use super::constants::*;
#[path = "identity/durable_file/mod.rs"]
mod durable_file;
use anyhow::Result;
pub use durable_file::DurableFileVersion;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::fs::Metadata;
#[cfg(target_os = "macos")]
use std::os::macos::fs::MetadataExt as MacMetadataExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
pub use yolo_durable_identity::DurableObjectIdentity;
#[path = "file/file_identity_compact.rs"]
pub mod compact;
#[path = "file/file_identity_descriptor.rs"]
mod file_identity_descriptor;
#[path = "file/file_identity_file.rs"]
mod file_identity_file;
#[path = "file/file_identity_stat.rs"]
mod file_identity_stat;
#[path = "object_identity_file.rs"]
mod object_identity_file;

#[cfg(all(unix, not(target_os = "macos")))]
const UNAVAILABLE_TIMESTAMP_COMPONENT: i64 = 0;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectIdentity {
    device: u64,
    inode: u64,
    // A persisted intent outlives its original descriptor; birth closes inode-reuse ABA.
    birth: FileTimestamp,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileIdentity {
    device: u64,
    inode: u64,
    length: u64,
    modified: FileTimestamp,
    changed: FileTimestamp,
    birth: FileTimestamp,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileTimestamp {
    seconds: i64,
    nanoseconds: i64,
}

impl FileTimestamp {
    pub(super) fn new(seconds: i64, nanoseconds: i64) -> Self {
        Self {
            seconds,
            nanoseconds,
        }
    }
}

impl FileIdentity {
    pub fn inspect_bounded(metadata: &Metadata, path: &Path, max_bytes: u64) -> Result<Self> {
        let identity = Self::inspect_base(metadata, path)?;
        anyhow::ensure!(
            metadata.mode() & PERMISSION_MODE_MASK == PRIVATE_FILE_MODE,
            "state permissions are unsafe: {}",
            path.display()
        );
        anyhow::ensure!(
            metadata.len() <= max_bytes,
            "state exceeds byte limit: {}",
            path.display()
        );
        Ok(identity)
    }

    pub fn verify_bounded(
        &self,
        metadata: &Metadata,
        length: u64,
        path: &Path,
        max_bytes: u64,
    ) -> Result<()> {
        let current = Self::inspect_bounded(metadata, path, max_bytes)?;
        anyhow::ensure!(
            current.unchanged(*self) && current.length == length,
            crate::ReadVersionChanged::state(path)
        );
        Ok(())
    }

    pub fn length(self) -> u64 {
        self.length
    }

    pub fn same_object(self, other: Self) -> bool {
        self.device == other.device && self.inode == other.inode && self.birth == other.birth
    }

    pub fn object_identity(self) -> ObjectIdentity {
        ObjectIdentity {
            device: self.device,
            inode: self.inode,
            birth: self.birth,
        }
    }

    pub fn same_content_version(self, other: Self) -> bool {
        self.same_object(other) && self.length == other.length && self.modified == other.modified
    }

    pub fn content_version_evidence(self) -> (u64, u64, u64, FileTimestamp, FileTimestamp) {
        (
            self.device,
            self.inode,
            self.length,
            self.modified,
            self.birth,
        )
    }

    pub fn unchanged(self, other: Self) -> bool {
        self.same_content_version(other) && self.changed == other.changed
    }

    fn inspect_base(metadata: &Metadata, path: &Path) -> Result<Self> {
        anyhow::ensure!(
            metadata.is_file(),
            "state is not a regular file: {}",
            path.display()
        );
        anyhow::ensure!(
            metadata.uid() == unsafe { libc::geteuid() },
            "state owner mismatch: {}",
            path.display()
        );
        anyhow::ensure!(
            metadata.nlink() == EXPECTED_FILE_LINK_COUNT,
            "state has unexpected hard links: {}",
            path.display()
        );
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            modified: FileTimestamp::new(metadata.mtime(), metadata.mtime_nsec()),
            changed: FileTimestamp::new(metadata.ctime(), metadata.ctime_nsec()),
            birth: birth_timestamp(metadata),
        })
    }
}

impl ObjectIdentity {
    pub fn from_metadata(metadata: &Metadata) -> Self {
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            birth: birth_timestamp(metadata),
        }
    }

    pub fn from_file(file: &File) -> Result<Option<Self>> {
        object_identity_file::from_file(file)
    }

    #[cfg(any(target_os = "linux", test))]
    pub(super) fn from_components(
        device: u64,
        inode: u64,
        birth_seconds: i64,
        birth_nanoseconds: i64,
    ) -> Self {
        Self {
            device,
            inode,
            birth: FileTimestamp::new(birth_seconds, birth_nanoseconds),
        }
    }
}

#[cfg(target_os = "macos")]
fn birth_timestamp(metadata: &Metadata) -> FileTimestamp {
    FileTimestamp::new(
        MacMetadataExt::st_birthtime(metadata),
        MacMetadataExt::st_birthtime_nsec(metadata),
    )
}

#[cfg(all(unix, not(target_os = "macos")))]
fn birth_timestamp(_metadata: &Metadata) -> FileTimestamp {
    FileTimestamp::new(
        UNAVAILABLE_TIMESTAMP_COMPONENT,
        UNAVAILABLE_TIMESTAMP_COMPONENT,
    )
}
