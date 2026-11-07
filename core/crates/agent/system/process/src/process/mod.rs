use super::ObservedProcess;
use crate::process::ProcessIdentity;
use anyhow::Result;

#[cfg(target_os = "macos")]
#[path = "budget.rs"]
mod budget;
#[cfg(target_os = "linux")]
#[path = "fallback.rs"]
mod fallback;
#[cfg(any(target_os = "linux", test))]
#[path = "fallback_callback.rs"]
mod fallback_callback;
#[cfg(any(target_os = "linux", test))]
#[path = "fallback_inventory.rs"]
mod fallback_inventory;
#[cfg(any(target_os = "linux", test))]
#[path = "fallback_lineage.rs"]
mod fallback_lineage;
#[cfg(target_os = "linux")]
#[path = "fallback_linux_identity.rs"]
mod fallback_linux_identity;
#[cfg(any(target_os = "linux", test))]
#[path = "fallback_linux_stat.rs"]
mod fallback_linux_stat;
#[cfg(any(target_os = "linux", test))]
#[path = "fallback_observation.rs"]
mod fallback_observation;
#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod macos;
#[cfg(target_os = "macos")]
#[path = "macos_ancestry.rs"]
mod macos_ancestry;
#[cfg(target_os = "macos")]
#[path = "macos_evidence.rs"]
mod macos_evidence;
#[cfg(target_os = "macos")]
#[path = "macos_observe.rs"]
mod macos_observe;

#[derive(Clone, Debug, Eq, PartialEq)]
#[doc(hidden)]
pub enum FencedDescendants {
    Stable(Vec<ObservedProcess>),
    RootChanged,
}

pub trait ProcessSource: Send + Sync {
    fn descendants(&self, root_pid: u32) -> Result<Vec<ObservedProcess>>;

    /// Observe the authenticated callback peer's ancestry without broadening agent discovery.
    fn fenced_callback_lineage(
        &self,
        root: ProcessIdentity,
        _peer_pid: u32,
    ) -> Result<FencedDescendants> {
        self.fenced_descendants(root)
    }

    /// Observe the callback peer's ancestry to the custody shell for providers
    /// that spawn callbacks in their own session and PTY. Foreground-group
    /// agreement cannot be required there; the pid/start-time chain and the
    /// provider-specific environment checks remain the fence.
    fn fenced_callback_ancestry(
        &self,
        root: ProcessIdentity,
        _peer_pid: u32,
    ) -> Result<FencedDescendants> {
        self.fenced_descendants(root)
    }

    fn fenced_descendants(&self, root: ProcessIdentity) -> Result<FencedDescendants> {
        if !root.matches() {
            return Ok(FencedDescendants::RootChanged);
        }
        let descendants = self.descendants(root.pid)?;
        if !root.matches() {
            return Ok(FencedDescendants::RootChanged);
        }
        Ok(FencedDescendants::Stable(descendants))
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeProcessSource {
    environment_keys: std::sync::Arc<[String]>,
}

impl RuntimeProcessSource {
    pub fn new(keys: impl IntoIterator<Item = &'static str>) -> Self {
        Self::from_owned(keys.into_iter().map(str::to_owned))
    }

    /// Keys supplied at runtime (for example by the isolated snapshot helper)
    /// live for the source, not for the process binary.
    pub fn from_owned(keys: impl IntoIterator<Item = String>) -> Self {
        Self {
            environment_keys: keys.into_iter().collect::<Vec<_>>().into(),
        }
    }

    fn key_refs(&self) -> Vec<&str> {
        self.environment_keys.iter().map(String::as_str).collect()
    }
}

impl ProcessSource for RuntimeProcessSource {
    fn fenced_callback_lineage(
        &self,
        root: ProcessIdentity,
        peer_pid: u32,
    ) -> Result<FencedDescendants> {
        #[cfg(target_os = "linux")]
        return fallback_callback::fenced_with_keys(root, peer_pid, &self.key_refs());
        #[cfg(not(target_os = "linux"))]
        {
            let _ = peer_pid;
            self.fenced_descendants(root)
        }
    }

    fn fenced_callback_ancestry(
        &self,
        root: ProcessIdentity,
        peer_pid: u32,
    ) -> Result<FencedDescendants> {
        #[cfg(target_os = "linux")]
        return fallback_callback::fenced_ancestry_with_keys(root, peer_pid, &self.key_refs());
        #[cfg(target_os = "macos")]
        return macos_ancestry::ancestry_with_keys(root, peer_pid, &self.key_refs());
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            let _ = (root, peer_pid);
            anyhow::bail!(super::constants::process_error::PLATFORM_UNSUPPORTED)
        }
    }

    fn descendants(&self, root_pid: u32) -> Result<Vec<ObservedProcess>> {
        #[cfg(target_os = "macos")]
        return macos::descendants_with_keys(root_pid, &self.key_refs());
        #[cfg(target_os = "linux")]
        return fallback::descendants_with_keys(root_pid, &self.key_refs());
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        anyhow::bail!(super::constants::process_error::PLATFORM_UNSUPPORTED)
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/process_tests.rs"]
mod tests;
