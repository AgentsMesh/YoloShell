use super::PrivateFileLock;
use anyhow::{Context, Result, ensure};
use std::os::unix::fs::MetadataExt;

const PROOF_CONTEXT: &str = "prove retained private file was unlinked";
const REMOVED_LINK_COUNT: u64 = 0;

pub(super) struct UnlinkProof {
    device: u64,
    inode: u64,
    mode: u32,
    owner: u32,
    group: u32,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl PrivateFileLock {
    pub(super) fn capture_unlink_proof(&self) -> Result<UnlinkProof> {
        crate::private_acl::require_empty(&self.file, self.entry.path()).context(PROOF_CONTEXT)?;
        UnlinkProof::capture(&self.file)
    }

    pub(super) fn verify_unlinked(&self, proof: UnlinkProof) -> Result<()> {
        crate::private_acl::require_empty(&self.file, self.entry.path()).context(PROOF_CONTEXT)?;
        proof.verify(&self.file)
    }
}

impl UnlinkProof {
    fn capture(file: &std::fs::File) -> Result<Self> {
        let metadata = file.metadata().context(PROOF_CONTEXT)?;
        ensure!(
            metadata.nlink() == crate::paths::constants::EXPECTED_FILE_LINK_COUNT,
            PROOF_CONTEXT
        );
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            owner: metadata.uid(),
            group: metadata.gid(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        })
    }

    fn verify(self, file: &std::fs::File) -> Result<()> {
        let metadata = file.metadata().context(PROOF_CONTEXT)?;
        ensure!(metadata.nlink() == REMOVED_LINK_COUNT, PROOF_CONTEXT);
        ensure!(metadata.dev() == self.device, PROOF_CONTEXT);
        ensure!(metadata.ino() == self.inode, PROOF_CONTEXT);
        ensure!(metadata.mode() == self.mode, PROOF_CONTEXT);
        ensure!(metadata.uid() == self.owner, PROOF_CONTEXT);
        ensure!(metadata.gid() == self.group, PROOF_CONTEXT);
        ensure!(metadata.len() == self.length, PROOF_CONTEXT);
        ensure!(metadata.mtime() == self.modified_seconds, PROOF_CONTEXT);
        ensure!(
            metadata.mtime_nsec() == self.modified_nanoseconds,
            PROOF_CONTEXT
        );
        ensure!(
            (metadata.ctime(), metadata.ctime_nsec())
                >= (self.changed_seconds, self.changed_nanoseconds),
            PROOF_CONTEXT
        );
        Ok(())
    }
}
