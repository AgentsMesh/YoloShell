use super::*;

const PLATFORM_UNAVAILABLE_DETAIL: &str = "PTY secondary-custody platform bundle is unavailable";

impl SecondaryCustodyLease {
    pub(super) fn platform_closure_settled(&self) -> anyhow::Result<bool> {
        super::super::lock_recover(&self.platform)
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(PLATFORM_UNAVAILABLE_DETAIL))?
            .closure_settled()
    }

    pub(super) fn platform_closure_settled_for(
        &self,
        settlement: &crate::pty_custody::CustodyReleaseSettlement,
    ) -> anyhow::Result<bool> {
        super::super::lock_recover(&self.platform)
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(PLATFORM_UNAVAILABLE_DETAIL))?
            .closure_settled_for(settlement)
    }

    pub(super) fn confirm_platform_root_exit(&self) -> anyhow::Result<()> {
        super::super::lock_recover(&self.platform)
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(PLATFORM_UNAVAILABLE_DETAIL))?
            .confirm_root_exit()
    }

    #[cfg(all(target_os = "macos", any(test, feature = "custody-release-test-hooks")))]
    pub(super) fn master_fd_custody_probe(
        &self,
    ) -> anyhow::Result<Option<crate::process::PtyMasterFdCustodyProbe>> {
        super::super::lock_recover(&self.platform)
            .as_ref()
            .map(PlatformSecondaryCustody::master_fd_custody_probe)
            .transpose()
            .map(Option::flatten)
    }

    pub(super) fn has_platform(&self) -> bool {
        super::super::lock_recover(&self.platform).is_some()
    }

    #[cfg(test)]
    pub(super) fn retains_platform(&self) -> bool {
        self.has_platform()
    }
}
