use super::LiveProcessIdentity;
use anyhow::Result;
use std::fmt::{Display, Formatter};
use std::time::Instant;
#[path = "signal_authority_exact.rs"]
mod exact;
#[cfg(target_os = "linux")]
#[path = "signal_authority_linux.rs"]
mod linux;
pub(super) use exact::ExactSignalOutcome;
const DESTRUCTIVE_SESSION_UNAVAILABLE_DETAIL: &str =
    "safe process-tree termination requires retained platform authority";
#[derive(PartialEq, Eq)]
pub struct DestructiveSessionUnavailable {
    _private: (),
}
impl Clone for DestructiveSessionUnavailable {
    fn clone(&self) -> Self {
        Self::new()
    }
}

pub(super) struct DestructiveSessionAccess {
    authority: Option<DestructiveSessionAuthority>,
    #[cfg(target_os = "linux")]
    containment: Option<super::linux_containment::LinuxContainmentLease>,
    #[cfg(target_os = "linux")]
    containment_removed: bool,
}

pub(super) struct DestructiveSessionAuthority(ProcessSignalAuthority);
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ForegroundSignalAuthority(ProcessSignalAuthority);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::process) struct ProcessSignalAuthority {
    _private: (),
}
impl DestructiveSessionAccess {
    pub(super) fn capture() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self {
                authority: session_destructive_capability_available()
                    .then(|| DestructiveSessionAuthority(ProcessSignalAuthority::new())),
            }
        }
        #[cfg(target_os = "linux")]
        {
            Self {
                authority: None,
                containment: None,
                containment_removed: false,
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Self { authority: None }
        }
    }

    pub(super) fn require(
        &self,
    ) -> std::result::Result<&DestructiveSessionAuthority, DestructiveSessionUnavailable> {
        self.authority
            .as_ref()
            .ok_or_else(DestructiveSessionUnavailable::new)
    }

    #[cfg(target_os = "linux")]
    pub(super) fn install_linux_containment(
        &mut self,
        containment: super::linux_containment::LinuxContainmentLease,
    ) -> Result<()> {
        anyhow::ensure!(
            self.containment.is_none() && !self.containment_removed,
            "Linux containment is already installed"
        );
        self.containment = Some(containment);
        self.authority = Some(DestructiveSessionAuthority(ProcessSignalAuthority::new()));
        Ok(())
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(super) fn authorize_for_test(&mut self) {
        self.authority = Some(DestructiveSessionAuthority(ProcessSignalAuthority::new()));
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(super) fn deny_for_test(&mut self) {
        self.authority = None;
    }
}

#[cfg(target_os = "macos")]
fn session_destructive_capability_available() -> bool {
    let available = super::macos_signal::capability_available();
    #[cfg(feature = "adversarial-test-hooks")]
    let available = super::authority_test_hook::destructive_capability(available);
    available
}

pub fn require_pty_spawn_signal_capability()
-> std::result::Result<(), DestructiveSessionUnavailable> {
    #[cfg(target_os = "macos")]
    if !super::macos_signal::capability_available() {
        return Err(DestructiveSessionUnavailable::new());
    }
    Ok(())
}

impl DestructiveSessionAuthority {
    pub(super) fn signal(&self, identity: &LiveProcessIdentity, signal: i32) -> Result<bool> {
        self.0.signal(identity, signal, None)
    }
}

impl ForegroundSignalAuthority {
    pub(super) fn verified() -> Self {
        Self(ProcessSignalAuthority::new())
    }

    pub(super) fn signal(&self, identity: &LiveProcessIdentity, signal: i32) -> Result<bool> {
        self.0.signal(identity, signal, None)
    }
}

pub(super) fn request_waitable_root_graceful_termination(
    identity: &LiveProcessIdentity,
) -> Result<bool> {
    let authority = ProcessSignalAuthority::new();
    authority.signal(identity, libc::SIGHUP, None)
}

#[derive(Clone, Copy)]
pub(super) enum ExactProcessSignal {
    ForceTermination,
    ResumeSuspended,
    ResumeSuspendedBefore(Instant),
}

pub(super) fn signal_exact_process(
    identity: &LiveProcessIdentity,
    request: ExactProcessSignal,
) -> Result<bool> {
    let (signal, deadline) = match request {
        ExactProcessSignal::ForceTermination => (libc::SIGKILL, None),
        ExactProcessSignal::ResumeSuspended => (libc::SIGCONT, None),
        ExactProcessSignal::ResumeSuspendedBefore(deadline) => (libc::SIGCONT, Some(deadline)),
    };
    ProcessSignalAuthority::new().signal(identity, signal, deadline)
}

impl ProcessSignalAuthority {
    pub fn new() -> Self {
        Self { _private: () }
    }

    fn signal(
        &self,
        identity: &LiveProcessIdentity,
        signal: i32,
        deadline: Option<Instant>,
    ) -> Result<bool> {
        identity.signal_authorized(self, signal, deadline)
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(in crate::process) fn for_test() -> Self {
        Self::new()
    }
}

impl DestructiveSessionUnavailable {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl std::fmt::Debug for DestructiveSessionUnavailable {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(DESTRUCTIVE_SESSION_UNAVAILABLE_DETAIL)
    }
}

impl Display for DestructiveSessionUnavailable {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(DESTRUCTIVE_SESSION_UNAVAILABLE_DETAIL)
    }
}

impl std::error::Error for DestructiveSessionUnavailable {}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/signal/signal_authority_tests.rs"]
mod tests;
