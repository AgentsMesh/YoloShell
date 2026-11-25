use super::ProcessIdentity;
use anyhow::Result;
use std::hash::{Hash, Hasher};
#[cfg(target_os = "linux")]
use std::sync::Arc;

#[path = "live_identity_control.rs"]
mod control;
#[cfg(target_os = "linux")]
#[path = "live_identity/linux_pidfd.rs"]
mod linux_pidfd;
pub(in crate::process) use control::ProcessExecutionState;
#[cfg(target_os = "macos")]
#[path = "live_identity_stop_macos.rs"]
mod macos_stop;
#[cfg(all(target_os = "macos", any(all(test, feature = "yolo-integration-tests"), feature = "runtime-component-tests")))]
#[path = "../../test_support/live_identity/live_identity_macos.rs"]
#[cfg(any(test, feature = "adversarial-test-hooks"))]
mod macos_fixture;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/live_identity/live_identity_hooks.rs"]
mod test_hooks;

#[cfg(target_os = "macos")]
const LIVE_PID_INVALID_DETAIL: &str = "live process id must be positive";
#[cfg(target_os = "macos")]
const LIVE_UNIQUE_ID_INVALID_DETAIL: &str = "live process unique id must be positive";
#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub use test_hooks::{
    install_test_live_execution_state_failure, install_test_live_signal_failure,
    install_test_live_signal_observer, install_test_match_failure,
};

#[derive(Clone, Debug)]
pub struct LiveProcessIdentity {
    pid: u32,
    generation: u64,
    #[cfg(target_os = "linux")]
    authority: Arc<linux_pidfd::RetainedPidfd>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct LiveProcessIdentityKey {
    pid: u32,
    generation: u64,
}

#[cfg(not(target_os = "linux"))]
impl Copy for LiveProcessIdentity {}

impl PartialEq for LiveProcessIdentity {
    fn eq(&self, other: &Self) -> bool {
        (self.pid, self.generation) == (other.pid, other.generation)
    }
}

impl Eq for LiveProcessIdentity {}

impl Hash for LiveProcessIdentity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pid.hash(state);
        self.generation.hash(state);
    }
}

impl LiveProcessIdentity {
    pub(super) fn key(&self) -> LiveProcessIdentityKey {
        LiveProcessIdentityKey {
            pid: self.pid,
            generation: self.generation,
        }
    }

    #[cfg(target_os = "linux")]
    pub(super) fn durable_projection(&self) -> ProcessIdentity {
        ProcessIdentity {
            pid: self.pid,
            start_time: self.generation,
        }
    }

    pub(crate) fn capture(identity: ProcessIdentity) -> Result<Option<Self>> {
        #[cfg(target_os = "linux")]
        {
            let Some(authority) = linux_pidfd::RetainedPidfd::capture(identity)? else {
                return Ok(None);
            };
            return Ok(Some(Self {
                pid: identity.pid,
                generation: identity.start_time,
                authority: Arc::new(authority),
            }));
        }
        #[cfg(target_os = "macos")]
        {
            let Some(metadata) = super::macos_kernel::metadata(identity.pid)? else {
                return Ok(None);
            };
            if metadata.start_time != identity.start_time {
                return Ok(None);
            }
            return Self::from_kernel(metadata.pid, metadata.unique_id).map(Some);
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Ok(identity.matches().then_some(Self {
                pid: identity.pid,
                generation: identity.start_time,
            }))
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn from_kernel(pid: u32, unique_id: u64) -> Result<Self> {
        anyhow::ensure!(pid != u32::default(), LIVE_PID_INVALID_DETAIL);
        anyhow::ensure!(unique_id != u64::default(), LIVE_UNIQUE_ID_INVALID_DETAIL);
        Ok(Self {
            pid,
            generation: unique_id,
        })
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub(super) fn from_portable(identity: ProcessIdentity) -> Self {
        Self {
            pid: identity.pid,
            generation: identity.start_time,
        }
    }

    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub(super) fn lineage_id(&self) -> u64 {
        #[cfg(target_os = "macos")]
        {
            self.generation
        }
        #[cfg(not(target_os = "macos"))]
        {
            u64::from(self.pid)
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn unique_id(&self) -> u64 {
        self.generation
    }

    #[cfg(test)]
    pub(super) fn with_generation(&self, generation: u64) -> Self {
        Self {
            generation,
            ..self.clone()
        }
    }

    pub fn try_matches(&self) -> Result<bool> {
        #[cfg(any(test, feature = "adversarial-test-hooks"))]
        if let Some(detail) = test_hooks::match_failure() {
            anyhow::bail!(detail);
        }
        #[cfg(target_os = "macos")]
        {
            Ok(super::macos_kernel::metadata(self.pid)?
                .is_some_and(|metadata| metadata.unique_id == self.generation))
        }
        #[cfg(target_os = "linux")]
        {
            self.authority.try_matches(self.pid, self.generation)
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Ok(super::process_identity_matches(self.pid, self.generation))
        }
    }

    #[cfg(any(test, feature = "runtime-component-tests", not(target_os = "macos")))]
    pub(crate) fn matches(&self) -> bool {
        self.try_matches().unwrap_or(false)
    }
}
