use super::LiveProcessIdentity;
use super::macos_kernel::{self, KernelProcessMetadata};
use super::signal_authority::{ExactSignalOutcome, ProcessSignalAuthority};
use super::signal_hook::run_pre_signal_hook;
use anyhow::{Context, Result, ensure};
use std::ffi::{c_char, c_void};
use std::mem::size_of;
use std::sync::OnceLock;
use std::time::Instant;

const AUDIT_TOKEN_WORDS: usize = 8;
const AUDIT_TOKEN_PID_INDEX: usize = 5;
const AUDIT_TOKEN_PID_VERSION_INDEX: usize = 7;
const AUDIT_TOKEN_SIZE: usize = 32;
const ATOMIC_SIGNAL_MAX_ATTEMPTS: usize = 8;
const SIGNAL_DELIVERED_RESULT: libc::c_int = 0;
const SIGNAL_SYMBOL: &[u8] = b"proc_signal_with_audittoken\0";
const SIGNAL_CAPABILITY_MISSING: &str = "atomic macOS process signaling is unavailable";
const IDENTITY_CAPTURE_CONTEXT: &str = "capture atomic macOS process identity";
const SIGNAL_CONTEXT: &str = "signal exact macOS process identity";
const SIGNAL_RACE_EXHAUSTED: &str = "macOS process identity changed throughout signal retries";

type SignalFunction = unsafe extern "C" fn(*mut AuditToken, libc::c_int) -> libc::c_int;

type KernelProcessIdentity = KernelProcessMetadata;

#[repr(C)]
struct AuditToken {
    words: [u32; AUDIT_TOKEN_WORDS],
}

enum AtomicSignalOutcome {
    Delivered,
    IdentityChanged,
}

enum ExactIdentityCapture {
    Current(KernelProcessIdentity),
    ConfirmedAbsent,
    IdentityReplaced,
}

const _: () = {
    assert!(size_of::<AuditToken>() == AUDIT_TOKEN_SIZE);
};

pub(super) fn signal(
    _authority: &ProcessSignalAuthority,
    identity: LiveProcessIdentity,
    signal: i32,
    deadline: Option<Instant>,
) -> Result<bool> {
    signal_exact(identity, signal, deadline).map(|outcome| outcome == ExactSignalOutcome::Delivered)
}

pub(super) fn signal_exact_sigkill(
    _authority: &ProcessSignalAuthority,
    identity: LiveProcessIdentity,
) -> Result<ExactSignalOutcome> {
    signal_exact(identity, libc::SIGKILL, None)
}

fn signal_exact(
    identity: LiveProcessIdentity,
    signal: i32,
    deadline: Option<Instant>,
) -> Result<ExactSignalOutcome> {
    let mut kernel_identity = match capture_exact_identity(identity)? {
        ExactIdentityCapture::Current(identity) => identity,
        ExactIdentityCapture::ConfirmedAbsent => return Ok(ExactSignalOutcome::ConfirmedAbsent),
        ExactIdentityCapture::IdentityReplaced => return Ok(ExactSignalOutcome::IdentityReplaced),
    };
    run_pre_signal_hook();
    for _attempt in std::iter::repeat_n((), ATOMIC_SIGNAL_MAX_ATTEMPTS) {
        ensure!(
            deadline.is_none_or(|limit| Instant::now() < limit),
            super::scoped_waitable_child::WAIT_DEADLINE_EXPIRED
        );
        match signal_once(kernel_identity, signal)? {
            AtomicSignalOutcome::Delivered => return Ok(ExactSignalOutcome::Delivered),
            AtomicSignalOutcome::IdentityChanged => {
                kernel_identity = match capture_exact_identity(identity)? {
                    ExactIdentityCapture::Current(identity) => identity,
                    ExactIdentityCapture::ConfirmedAbsent => {
                        return Ok(ExactSignalOutcome::ConfirmedAbsent);
                    }
                    ExactIdentityCapture::IdentityReplaced => {
                        return Ok(ExactSignalOutcome::IdentityReplaced);
                    }
                };
            }
        }
    }
    anyhow::bail!(SIGNAL_RACE_EXHAUSTED)
}

#[cfg(any(test, feature = "runtime-component-tests"))]
fn capture_kernel_identity(identity: LiveProcessIdentity) -> Result<Option<KernelProcessIdentity>> {
    Ok(match capture_exact_identity(identity)? {
        ExactIdentityCapture::Current(identity) => Some(identity),
        ExactIdentityCapture::ConfirmedAbsent | ExactIdentityCapture::IdentityReplaced => None,
    })
}

fn capture_exact_identity(identity: LiveProcessIdentity) -> Result<ExactIdentityCapture> {
    let Some(metadata) =
        macos_kernel::metadata(identity.pid()).context(IDENTITY_CAPTURE_CONTEXT)?
    else {
        return Ok(ExactIdentityCapture::ConfirmedAbsent);
    };
    Ok(if metadata.unique_id == identity.unique_id() {
        ExactIdentityCapture::Current(metadata)
    } else {
        ExactIdentityCapture::IdentityReplaced
    })
}

fn signal_once(identity: KernelProcessIdentity, signal: i32) -> Result<AtomicSignalOutcome> {
    signal_once_with_capability(
        identity,
        signal,
        *SIGNAL_FUNCTION.get_or_init(resolve_signal_function),
    )
}

fn signal_once_with_capability(
    identity: KernelProcessIdentity,
    signal: i32,
    capability: Option<SignalFunction>,
) -> Result<AtomicSignalOutcome> {
    let function = capability.context(SIGNAL_CAPABILITY_MISSING)?;
    let mut token = AuditToken {
        words: [u32::default(); AUDIT_TOKEN_WORDS],
    };
    // XNU resolves this token atomically by its PID and pidversion words.
    token.words[AUDIT_TOKEN_PID_INDEX] = identity.pid;
    token.words[AUDIT_TOKEN_PID_VERSION_INDEX] = identity.pid_version;
    let result = unsafe { function(&mut token, signal) };
    if result == SIGNAL_DELIVERED_RESULT {
        return Ok(AtomicSignalOutcome::Delivered);
    }
    if result == libc::ESRCH {
        return Ok(AtomicSignalOutcome::IdentityChanged);
    }
    Err(std::io::Error::from_raw_os_error(result)).context(SIGNAL_CONTEXT)
}

fn resolve_signal_function() -> Option<SignalFunction> {
    let symbol =
        unsafe { libc::dlsym(libc::RTLD_DEFAULT, SIGNAL_SYMBOL.as_ptr().cast::<c_char>()) };
    if symbol.is_null() {
        return None;
    }
    const _: () = assert!(size_of::<SignalFunction>() == size_of::<*mut c_void>());
    Some(unsafe { std::mem::transmute::<*mut c_void, SignalFunction>(symbol) })
}

static SIGNAL_FUNCTION: OnceLock<Option<SignalFunction>> = OnceLock::new();

pub(super) fn capability_available() -> bool {
    #[cfg(any(test, feature = "runtime-component-tests"))]
    if let Some(available) = TEST_CAPABILITY.with(std::cell::Cell::get) {
        return available;
    }
    SIGNAL_FUNCTION
        .get_or_init(resolve_signal_function)
        .is_some()
}

#[cfg(any(test, feature = "runtime-component-tests"))]
thread_local! {
    static TEST_CAPABILITY: std::cell::Cell<Option<bool>> = const {
        std::cell::Cell::new(None)
    };
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(crate) fn with_test_capability<T>(available: bool, operation: impl FnOnce() -> T) -> T {
    TEST_CAPABILITY.with(|slot| {
        let previous = slot.replace(Some(available));
        let result = operation();
        slot.set(previous);
        result
    })
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_signal_tests.rs"]
mod tests;
