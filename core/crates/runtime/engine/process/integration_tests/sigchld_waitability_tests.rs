use super::{normalize_sigchld_waitability, read_sigchld_action, set_sigchld_action};
use crate::process::{DetachedStandardError, spawn_detached};
use crate::runtime_constants::TEST_HARNESS_EXACT_ARGUMENT;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::path::Path;
use std::process::{Command, ExitStatus};
use std::sync::{Mutex, MutexGuard};

const ROLE_ENVIRONMENT: &str = "YOLO_SIGCHLD_TEST_ROLE";
const VERIFY_ROLE: &str = "verify";
const TEST_CHILD_EXECUTABLE: &str = crate::test_executables::TRUE;
const IGNORED_TEST_NAME: &str =
    "sigchld_waitability::tests::inherited_sigchld_ignore_is_normalized";
const NOCLDWAIT_TEST_NAME: &str =
    "sigchld_waitability::tests::inherited_sigchld_nocldwait_is_normalized";
const DETACHED_SPAWN_TEST_NAME: &str =
    "sigchld_waitability::tests::detached_spawn_boundary_is_waitable";
const PTY_SPAWN_TEST_NAME: &str = "sigchld_waitability::tests::pty_spawn_boundary_is_waitable";
const HELPER_FAILURE: &str = "isolated SIGCHLD test helper failed";
const TEST_CHILD_WAIT_FAILURE: &str = "wait for waitability test child";
const UNKNOWN_ROLE_FAILURE: &str = "unexpected SIGCHLD test helper role";
const RESTORE_FAILURE: &str = "restore original SIGCHLD disposition";
const TEST_TERMINAL_ROWS: u16 = 24;
const TEST_TERMINAL_COLUMNS: u16 = 80;
const TEST_TERMINAL_PIXEL_SIZE: u16 = 0;

static TEST_DISPOSITION_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy)]
enum AdversarialDisposition {
    Ignored,
    NoChildWait,
}

struct SigchldActionGuard {
    original: libc::sigaction,
}

#[test]
fn inherited_sigchld_ignore_is_normalized() {
    run_isolated(
        IGNORED_TEST_NAME,
        AdversarialDisposition::Ignored,
        normalize_and_verify,
    );
}

#[test]
fn inherited_sigchld_nocldwait_is_normalized() {
    run_isolated(
        NOCLDWAIT_TEST_NAME,
        AdversarialDisposition::NoChildWait,
        normalize_and_verify,
    );
}

#[test]
fn detached_spawn_boundary_is_waitable() {
    run_isolated(
        DETACHED_SPAWN_TEST_NAME,
        AdversarialDisposition::Ignored,
        verify_detached_spawn,
    );
}

#[test]
fn pty_spawn_boundary_is_waitable() {
    run_isolated(
        PTY_SPAWN_TEST_NAME,
        AdversarialDisposition::NoChildWait,
        verify_pty_spawn,
    );
}

fn run_isolated(test_name: &str, disposition: AdversarialDisposition, verify: fn()) {
    match std::env::var(ROLE_ENVIRONMENT).ok().as_deref() {
        None => assert_helper_success(spawn_helper(test_name)),
        Some(VERIFY_ROLE) => with_disposition_guard(|guard| {
            install_adversarial(disposition, guard.original);
            verify();
            assert_waitable_default(read_sigchld_action().unwrap());
        }),
        Some(role) => panic!("{UNKNOWN_ROLE_FAILURE}: {role}"),
    }
}

fn normalize_and_verify() {
    normalize_sigchld_waitability().unwrap();
    assert_waitable_default(read_sigchld_action().unwrap());
    assert_child_remains_waitable();
}

fn verify_detached_spawn() {
    let mut child = spawn_detached(
        Path::new(TEST_CHILD_EXECUTABLE),
        &[],
        DetachedStandardError::Null,
    )
    .unwrap();
    assert!(child.wait().unwrap().success());
}

fn verify_pty_spawn() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: TEST_TERMINAL_ROWS,
            cols: TEST_TERMINAL_COLUMNS,
            pixel_width: TEST_TERMINAL_PIXEL_SIZE,
            pixel_height: TEST_TERMINAL_PIXEL_SIZE,
        })
        .unwrap();
    let command = CommandBuilder::new(TEST_CHILD_EXECUTABLE);
    let mut child =
        crate::pty_spawn_test_support::spawn_waitable_pty_root(pair.slave.as_ref(), command)
            .unwrap();
    assert!(child.wait().unwrap().success());
}

fn with_disposition_guard(test: impl FnOnce(&SigchldActionGuard)) {
    let _lock = lock_test_disposition();
    let guard = SigchldActionGuard {
        original: read_sigchld_action().unwrap(),
    };
    test(&guard);
}

fn install_adversarial(kind: AdversarialDisposition, mut action: libc::sigaction) {
    match kind {
        AdversarialDisposition::Ignored => action.sa_sigaction = libc::SIG_IGN,
        AdversarialDisposition::NoChildWait => {
            action.sa_sigaction = libc::SIG_DFL;
            action.sa_flags |= libc::SA_NOCLDWAIT | libc::SA_SIGINFO;
        }
    }
    set_sigchld_action(&action).unwrap();
    assert_adversarial(kind, read_sigchld_action().unwrap());
}

fn assert_adversarial(kind: AdversarialDisposition, action: libc::sigaction) {
    match kind {
        AdversarialDisposition::Ignored => assert_eq!(action.sa_sigaction, libc::SIG_IGN),
        AdversarialDisposition::NoChildWait => {
            assert_ne!(action.sa_flags & libc::SA_NOCLDWAIT, libc::c_int::default());
            assert_ne!(action.sa_flags & libc::SA_SIGINFO, libc::c_int::default());
        }
    }
}

fn assert_waitable_default(action: libc::sigaction) {
    assert_eq!(action.sa_sigaction, libc::SIG_DFL);
    assert_eq!(action.sa_flags & libc::SA_NOCLDWAIT, libc::c_int::default());
    assert_eq!(action.sa_flags & libc::SA_SIGINFO, libc::c_int::default());
}

fn assert_child_remains_waitable() {
    let mut child = crate::process::spawn_waitable_test_child(Command::new(TEST_CHILD_EXECUTABLE));
    let status = child.wait().expect(TEST_CHILD_WAIT_FAILURE);
    assert!(status.success());
}

fn spawn_helper(test_name: &str) -> ExitStatus {
    Command::new(std::env::current_exe().unwrap())
        .arg(TEST_HARNESS_EXACT_ARGUMENT)
        .arg(test_name)
        .env(ROLE_ENVIRONMENT, VERIFY_ROLE)
        .status()
        .unwrap()
}

fn assert_helper_success(status: ExitStatus) {
    assert!(status.success(), "{HELPER_FAILURE}: {status}");
}

fn lock_test_disposition() -> MutexGuard<'static, ()> {
    TEST_DISPOSITION_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Drop for SigchldActionGuard {
    fn drop(&mut self) {
        set_sigchld_action(&self.original).expect(RESTORE_FAILURE);
    }
}
