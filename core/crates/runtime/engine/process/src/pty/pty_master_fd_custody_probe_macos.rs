use super::pty_session_proof::{
    MacosPassivePtyMaster, MacosPtyObjectIdentity, capture_exact_descriptor_identity,
    has_stable_exact_descriptor,
};
use anyhow::{Context, Result};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::FileTypeExt;
use std::path::PathBuf;

const MASTER_DESCRIPTOR_IDENTITY_CONTEXT: &str = "capture passive PTY master descriptor identity";
const MASTER_DESCRIPTOR_IDENTITY_MISSING: &str =
    "passive PTY master descriptor has no vnode identity";
#[cfg(any(test, feature = "adversarial-test-hooks"))]
const TEST_IDENTITY_INODE_DELTA: u64 = 1;

#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PtyMasterFdCustodyProbe {
    holder_pid: u32,
    holder_start_time: u64,
    descriptor: RawFd,
    descriptor_identity: MacosPtyObjectIdentity,
    pty_path: PathBuf,
    pty_identity: MacosPtyObjectIdentity,
}

impl PtyMasterFdCustodyProbe {
    pub(super) fn capture(master: &MacosPassivePtyMaster) -> Result<Self> {
        let holder = super::ProcessIdentity::capture(std::process::id())?;
        let descriptor = master.descriptor().as_raw_fd();
        let descriptor_identity = capture_exact_descriptor_identity(holder.pid, descriptor)
            .context(MASTER_DESCRIPTOR_IDENTITY_CONTEXT)?
            .context(MASTER_DESCRIPTOR_IDENTITY_MISSING)?;
        Ok(Self {
            holder_pid: holder.pid,
            holder_start_time: holder.start_time,
            descriptor,
            descriptor_identity,
            pty_path: master.slave_path().to_path_buf(),
            pty_identity: master.object_identity(),
        })
    }

    pub fn process_holds_pty_object(&self, pid: u32) -> Result<bool> {
        if pid != self.holder_pid
            || !super::process_identity_matches(self.holder_pid, self.holder_start_time)
        {
            return Ok(false);
        }
        Ok(self.pty_path_matches()?
            && has_stable_exact_descriptor(pid, self.descriptor, self.descriptor_identity)?)
    }

    #[doc(hidden)]
    pub fn confirm_exact(&self) -> Result<()> {
        anyhow::ensure!(
            self.process_holds_pty_object(self.holder_pid)?,
            MASTER_DESCRIPTOR_IDENTITY_MISSING
        );
        Ok(())
    }

    #[cfg(test)]
    pub(in crate::process) fn resilience_proof(&self) -> Result<super::PtySessionProof> {
        let issuer = super::MacosProcessCapabilityIssuer::create()?;
        let (reader, _writer) = issuer.transfer_for_test()?;
        super::PtySessionProof::capture_custody(self.clone(), reader)
    }

    pub(in crate::process) fn pty_path(&self) -> &std::path::Path {
        &self.pty_path
    }

    pub(in crate::process) fn pty_object_identity(&self) -> MacosPtyObjectIdentity {
        self.pty_identity
    }

    fn pty_path_matches(&self) -> Result<bool> {
        let metadata = match std::fs::metadata(&self.pty_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        Ok(metadata.file_type().is_char_device()
            && MacosPtyObjectIdentity::from_metadata(&metadata) == self.pty_identity)
    }

    #[cfg(test)]
    pub(crate) fn capture_for_test(master: &MacosPassivePtyMaster) -> Result<Self> {
        Self::capture(master)
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    #[doc(hidden)]
    pub fn rebind_holder_for_test(self, holder: super::ProcessIdentity) -> Self {
        Self {
            holder_pid: holder.pid,
            holder_start_time: holder.start_time,
            ..self
        }
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    #[doc(hidden)]
    pub fn corrupt_descriptor_identity_for_test(mut self) -> Self {
        self.descriptor_identity = super::pty_session_proof::corrupt_vnode_identity_for_test(
            self.descriptor_identity,
            TEST_IDENTITY_INODE_DELTA,
        );
        self
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/pty/master/pty_master_fd_custody_probe_macos_tests.rs"]
mod tests;
