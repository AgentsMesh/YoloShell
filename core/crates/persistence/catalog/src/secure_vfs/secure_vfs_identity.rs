use anyhow::Result;
use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

const EXPECTED_LINK_COUNT: u64 = 1;
const INTERNAL_ALIAS_LINK_COUNT: u64 = EXPECTED_LINK_COUNT + 1;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(super) struct FileIdentity {
    device: u64,
    inode: u64,
}

impl FileIdentity {
    pub(super) fn inspect(metadata: &Metadata, path: &Path) -> Result<Self> {
        verify_regular(metadata, EXPECTED_LINK_COUNT, path)?;
        if metadata.uid() != unsafe { libc::geteuid() } {
            return Err(unsafe_artifact(format!(
                "SQLite artifact owner mismatch: {}",
                path.display()
            )));
        }
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
        })
    }

    pub(super) fn verify_alias(
        &self,
        descriptor: &Metadata,
        alias: &Metadata,
        path: &Path,
    ) -> Result<()> {
        verify_regular(descriptor, INTERNAL_ALIAS_LINK_COUNT, path)?;
        verify_regular(alias, INTERNAL_ALIAS_LINK_COUNT, path)?;
        if !self.matches(alias) {
            return Err(unsafe_artifact(format!(
                "SQLite alias inode mismatch: {}",
                path.display()
            )));
        }
        Ok(())
    }

    pub(super) fn verify_final(
        &self,
        descriptor: &Metadata,
        path_metadata: &Metadata,
        path: &Path,
    ) -> Result<()> {
        verify_regular(descriptor, EXPECTED_LINK_COUNT, path)?;
        verify_regular(path_metadata, EXPECTED_LINK_COUNT, path)?;
        if !self.matches(path_metadata) {
            return Err(unsafe_artifact(format!(
                "SQLite artifact changed during xOpen: {}",
                path.display()
            )));
        }
        Ok(())
    }

    pub(super) fn device(self) -> u64 {
        self.device
    }

    fn matches(&self, metadata: &Metadata) -> bool {
        metadata.dev() == self.device && metadata.ino() == self.inode
    }
}

fn verify_regular(metadata: &Metadata, links: u64, path: &Path) -> Result<()> {
    if !metadata.file_type().is_file() {
        return Err(unsafe_artifact(format!(
            "SQLite artifact is not regular: {}",
            path.display()
        )));
    }
    if metadata.nlink() != links {
        return Err(unsafe_artifact(format!(
            "SQLite artifact link count changed: {}",
            path.display()
        )));
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct UnsafeSqliteArtifact(String);

pub(super) fn unsafe_artifact(message: String) -> anyhow::Error {
    UnsafeSqliteArtifact(message).into()
}

pub(super) fn is_unsafe_artifact(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| cause.is::<UnsafeSqliteArtifact>())
}
