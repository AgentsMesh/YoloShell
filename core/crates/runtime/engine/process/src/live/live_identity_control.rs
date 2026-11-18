use super::LiveProcessIdentity;
use crate::process::signal_authority::{ExactSignalOutcome, ProcessSignalAuthority};
use anyhow::Result;
use std::time::Instant;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
const ATOMIC_SIGNAL_UNAVAILABLE_DETAIL: &str =
    "atomic process signaling is unavailable on this platform";

#[derive(Debug, Eq, PartialEq)]
pub(in crate::process) enum ProcessExecutionState {
    Running,
    Stopped,
    Gone,
}

impl LiveProcessIdentity {
    pub(in crate::process) fn execution_state(&self) -> Result<ProcessExecutionState> {
        #[cfg(any(test, feature = "adversarial-test-hooks"))]
        if let Some(error) = super::test_hooks::execution_state_failure() {
            return Err(error);
        }
        #[cfg(target_os = "macos")]
        {
            return super::macos_stop::execution_state(*self);
        }
        #[cfg(target_os = "linux")]
        {
            return self.authority.execution_state(self.pid, self.generation);
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Ok(if self.try_matches()? {
                ProcessExecutionState::Running
            } else {
                ProcessExecutionState::Gone
            })
        }
    }

    pub(in crate::process) fn signal_authorized(
        &self,
        authority: &ProcessSignalAuthority,
        signal: i32,
        deadline: Option<Instant>,
    ) -> Result<bool> {
        #[cfg(any(test, feature = "adversarial-test-hooks"))]
        super::test_hooks::observe_signal(signal);
        #[cfg(any(test, feature = "adversarial-test-hooks"))]
        if let Some(detail) = super::test_hooks::signal_failure() {
            anyhow::bail!(detail);
        }
        #[cfg(target_os = "macos")]
        {
            return crate::process::macos_signal::signal(authority, *self, signal, deadline);
        }
        #[cfg(target_os = "linux")]
        {
            let _ = deadline;
            return self
                .authority
                .signal(authority, self.pid, self.generation, signal);
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = (authority, signal, deadline);
            anyhow::bail!(ATOMIC_SIGNAL_UNAVAILABLE_DETAIL)
        }
    }

    pub(in crate::process) fn signal_exact_sigkill_authorized(
        &self,
        authority: &ProcessSignalAuthority,
    ) -> Result<ExactSignalOutcome> {
        #[cfg(any(test, feature = "adversarial-test-hooks"))]
        if let Some(detail) = super::test_hooks::signal_failure() {
            anyhow::bail!(detail);
        }
        #[cfg(target_os = "macos")]
        {
            return crate::process::macos_signal::signal_exact_sigkill(authority, *self);
        }
        #[cfg(target_os = "linux")]
        {
            return self
                .authority
                .signal_exact_sigkill(authority, self.pid, self.generation);
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = authority;
            anyhow::bail!(ATOMIC_SIGNAL_UNAVAILABLE_DETAIL)
        }
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(in crate::process) fn signal_for_test(&self, signal: i32) -> Result<bool> {
        self.signal_authorized(&ProcessSignalAuthority::for_test(), signal, None)
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(in crate::process) fn signal(&self, signal: i32) -> Result<bool> {
        self.signal_for_test(signal)
    }
}
