use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;

const EXPECTED_FILE_LINKS: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EvidenceKind {
    Directory,
    RegularFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EvidenceIdentity {
    device: u64,
    inode: u64,
    file_type: u32,
    owner: u32,
    links: u64,
    pub length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

impl EvidenceIdentity {
    pub(super) fn from_metadata(metadata: &Metadata, kind: EvidenceKind) -> Option<Self> {
        Self::new(
            metadata.dev(),
            metadata.ino(),
            metadata.mode(),
            metadata.uid(),
            metadata.nlink(),
            metadata.len(),
            metadata.mtime(),
            metadata.mtime_nsec(),
            metadata.ctime(),
            metadata.ctime_nsec(),
            kind,
        )
    }

    pub(super) fn from_stat(metadata: &libc::stat, kind: EvidenceKind) -> Option<Self> {
        Self::new(
            metadata.st_dev.try_into().ok()?,
            metadata.st_ino,
            u32::from(metadata.st_mode),
            metadata.st_uid,
            metadata.st_nlink.into(),
            metadata.st_size.try_into().ok()?,
            metadata.st_mtime,
            metadata.st_mtime_nsec,
            metadata.st_ctime,
            metadata.st_ctime_nsec,
            kind,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        device: u64,
        inode: u64,
        mode: u32,
        owner: u32,
        links: u64,
        length: u64,
        modified_seconds: i64,
        modified_nanoseconds: i64,
        changed_seconds: i64,
        changed_nanoseconds: i64,
        kind: EvidenceKind,
    ) -> Option<Self> {
        let expected_type = match kind {
            EvidenceKind::Directory => u32::from(libc::S_IFDIR),
            EvidenceKind::RegularFile => u32::from(libc::S_IFREG),
        };
        let file_type = mode & u32::from(libc::S_IFMT);
        if file_type != expected_type || owner != unsafe { libc::geteuid() } {
            return None;
        }
        if kind == EvidenceKind::RegularFile && links != EXPECTED_FILE_LINKS {
            return None;
        }
        Some(Self {
            device,
            inode,
            file_type,
            owner,
            links,
            length,
            modified_seconds,
            modified_nanoseconds,
            changed_seconds,
            changed_nanoseconds,
        })
    }

    pub(super) fn same_object(self, other: Self) -> bool {
        self.device == other.device
            && self.inode == other.inode
            && self.file_type == other.file_type
            && self.owner == other.owner
    }
}
