use super::{CodexExecutableKind, EXECUTABLE_INVALID};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use std::os::macos::fs::MetadataExt as MacMetadataExt;
use std::os::unix::fs::MetadataExt;

const EXECUTABLE_PERMISSION_MASK: u32 =
    libc::S_IXUSR as u32 | libc::S_IXGRP as u32 | libc::S_IXOTH as u32;
const FILE_TYPE_MASK: u32 = libc::S_IFMT as u32;
const REGULAR_FILE_TYPE: u32 = libc::S_IFREG as u32;
const ROOT_USER_ID: u32 = 0;
const UNTRUSTED_WRITE_MASK: u32 = libc::S_IWGRP as u32 | libc::S_IWOTH as u32;
const EXPECTED_LINK_COUNT: u64 = 1;
#[cfg(not(target_os = "macos"))]
const UNAVAILABLE_BIRTH_COMPONENT: i64 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexExecutableIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    owner: u32,
    group: u32,
    links: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
    birth_seconds: i64,
    birth_nanoseconds: i64,
}

impl CodexExecutableIdentity {
    pub(crate) fn inspect(metadata: &std::fs::Metadata, kind: CodexExecutableKind) -> Result<Self> {
        capture(metadata, kind)
    }

    pub(super) fn length(self) -> u64 {
        self.length
    }

    pub(super) fn validate(self, kind: CodexExecutableKind) -> Result<()> {
        let trusted_owner = self.owner == unsafe { libc::geteuid() } || self.owner == ROOT_USER_ID;
        ensure!(
            self.mode & FILE_TYPE_MASK == REGULAR_FILE_TYPE
                && self.mode & EXECUTABLE_PERMISSION_MASK != 0
                && self.mode & UNTRUSTED_WRITE_MASK == 0
                && trusted_owner
                && self.links == EXPECTED_LINK_COUNT
                && self.length > 0
                && self.length <= kind.maximum_bytes(),
            EXECUTABLE_INVALID
        );
        Ok(())
    }
}

pub(super) fn capture(
    metadata: &std::fs::Metadata,
    kind: CodexExecutableKind,
) -> Result<CodexExecutableIdentity> {
    let (birth_seconds, birth_nanoseconds) = birth(metadata);
    let value = CodexExecutableIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        owner: metadata.uid(),
        group: metadata.gid(),
        links: metadata.nlink(),
        length: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
        birth_seconds,
        birth_nanoseconds,
    };
    value.validate(kind)?;
    Ok(value)
}

#[cfg(target_os = "macos")]
fn birth(metadata: &std::fs::Metadata) -> (i64, i64) {
    (metadata.st_birthtime(), metadata.st_birthtime_nsec())
}

#[cfg(not(target_os = "macos"))]
fn birth(_metadata: &std::fs::Metadata) -> (i64, i64) {
    (UNAVAILABLE_BIRTH_COMPONENT, UNAVAILABLE_BIRTH_COMPONENT)
}
