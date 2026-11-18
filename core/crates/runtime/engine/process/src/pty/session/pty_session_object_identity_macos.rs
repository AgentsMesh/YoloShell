use std::os::unix::fs::MetadataExt;

const MISSING_PTY_OBJECT_IDENTITY_DETAIL: &str = "PTY object identity is unavailable";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Deserialize, serde::Serialize)]
pub(in crate::process) struct MacosPtyObjectIdentity {
    filesystem_device: u32,
    inode: u64,
    terminal_rdev: u32,
}

impl MacosPtyObjectIdentity {
    pub(in crate::process) fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        Self::from_kernel_fields(
            normalize_metadata_device(metadata.dev()),
            metadata.ino(),
            normalize_metadata_device(metadata.rdev()),
        )
    }

    pub(super) fn from_vnode_stat(stat: &libc::vinfo_stat) -> Self {
        Self::from_kernel_fields(stat.vst_dev, stat.vst_ino, stat.vst_rdev)
    }

    pub(super) fn from_stat(stat: &libc::stat) -> Self {
        Self::from_kernel_fields(stat.st_dev as u32, stat.st_ino, stat.st_rdev as u32)
    }

    pub(super) const fn from_kernel_fields(
        filesystem_device: u32,
        inode: u64,
        terminal_rdev: u32,
    ) -> Self {
        Self {
            filesystem_device,
            inode,
            terminal_rdev,
        }
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(super) const fn filesystem_device(self) -> u32 {
        self.filesystem_device
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(super) const fn inode(self) -> u64 {
        self.inode
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(super) const fn terminal_rdev(self) -> u32 {
        self.terminal_rdev
    }
}

fn normalize_metadata_device(device: u64) -> u32 {
    device as u32
}

impl super::PtySessionProof {
    pub(super) fn pty_object_identity(
        &self,
    ) -> super::macos_contract::CapabilityProofResult<MacosPtyObjectIdentity> {
        self.tty_object_identity.ok_or_else(|| {
            super::macos_contract::CapabilityProofError::invariant(anyhow::anyhow!(
                MISSING_PTY_OBJECT_IDENTITY_DETAIL
            ))
        })
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/session/object/pty_session_object_identity_macos_tests.rs"]
mod tests;
