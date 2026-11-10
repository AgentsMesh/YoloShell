use super::StartupChildGuard;
use anyhow::{Context, Result};

pub(crate) const STARTUP_TREE_UNAVAILABLE_CONTEXT: &str =
    "startup process tree is unavailable for PTY custody";

impl StartupChildGuard {
    #[cfg(target_os = "macos")]
    pub fn export_secondary_custody(
        &self,
        runtime: &crate::pty_custody::CustodyRuntimeIdentity,
    ) -> Result<crate::process::MacosSecondaryCustodyExport> {
        self.tree
            .as_ref()
            .context(STARTUP_TREE_UNAVAILABLE_CONTEXT)?
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .export_macos_secondary_custody(runtime)
    }

    #[cfg(target_os = "linux")]
    pub fn export_secondary_custody(
        &self,
        master: std::os::fd::RawFd,
    ) -> Result<crate::process::LinuxSecondaryCustodyExport> {
        self.tree
            .as_ref()
            .context(STARTUP_TREE_UNAVAILABLE_CONTEXT)?
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .export_linux_secondary_custody(master)
    }
}
