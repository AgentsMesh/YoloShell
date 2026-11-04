use super::PlatformSecondaryCustody;
use anyhow::Result;
#[cfg(target_os = "macos")]
use std::os::fd::BorrowedFd;

impl PlatformSecondaryCustody {
    #[cfg(target_os = "macos")]
    pub(super) fn guardian_descriptors(
        &self,
    ) -> Option<[BorrowedFd<'_>; crate::process::MACOS_SECONDARY_CUSTODY_DESCRIPTOR_COUNT]> {
        match self {
            Self::Active(custody) => Some(custody.as_descriptors()),
            Self::ReleasedSuccessor => None,
        }
    }

    pub(super) fn confirm_root_exit(&self) -> Result<()> {
        match self {
            #[cfg(target_os = "macos")]
            Self::Active(custody) => custody.confirm_root_exit(),
            #[cfg(target_os = "linux")]
            Self::Active(_) => Ok(()),
            Self::ReleasedSuccessor => Ok(()),
        }
    }

    pub(super) fn closure_settled(&self) -> Result<bool> {
        match self {
            Self::Active(custody) => custody.closure_settled(),
            Self::ReleasedSuccessor => Ok(true),
        }
    }

    pub(super) fn closure_settled_for(
        &self,
        settlement: &crate::pty_custody::CustodyReleaseSettlement,
    ) -> Result<bool> {
        match self {
            #[cfg(target_os = "macos")]
            Self::Active(custody) => custody.closure_settled_for(settlement),
            #[cfg(target_os = "linux")]
            Self::Active(custody) => custody.closure_settled(),
            Self::ReleasedSuccessor => Ok(true),
        }
    }

    pub(super) fn request_active_settlement(&self) -> Result<bool> {
        match self {
            Self::Active(custody) => custody.request_active_settlement(),
            Self::ReleasedSuccessor => Ok(true),
        }
    }

    #[cfg(all(target_os = "macos", any(test, feature = "custody-release-test-hooks")))]
    pub(super) fn master_fd_custody_probe(
        &self,
    ) -> Result<Option<crate::process::PtyMasterFdCustodyProbe>> {
        match self {
            Self::Active(custody) => custody.master_fd_custody_probe().map(Some),
            Self::ReleasedSuccessor => Ok(None),
        }
    }
}
