use std::fs::Metadata;
use std::io;
use std::path::Path;

const LOCAL_INPUT_CHANGED_MESSAGE: &str = "local input changed while reading";

#[cfg(unix)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileSnapshot {
    device: u64,
    inode: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
impl FileSnapshot {
    fn capture(metadata: &Metadata) -> Self {
        use std::os::unix::fs::MetadataExt;
        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        }
    }
}

pub(super) fn ensure_stable(
    path: &Path,
    before: &Metadata,
    after: &Metadata,
    read: usize,
) -> io::Result<()> {
    #[cfg(unix)]
    {
        let path_snapshot = FileSnapshot::capture(&std::fs::symlink_metadata(path)?);
        let before = FileSnapshot::capture(before);
        let after = FileSnapshot::capture(after);
        if !snapshots_match(before, after, path_snapshot, read) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                LOCAL_INPUT_CHANGED_MESSAGE,
            ));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn snapshots_match(
    before: FileSnapshot,
    after: FileSnapshot,
    path: FileSnapshot,
    read: usize,
) -> bool {
    before.device == after.device
        && before.inode == after.inode
        && before.length == after.length
        && before.modified_seconds == after.modified_seconds
        && before.modified_nanoseconds == after.modified_nanoseconds
        && before.changed_seconds == after.changed_seconds
        && before.changed_nanoseconds == after.changed_nanoseconds
        && after.length == read as u64
        && path.device == after.device
        && path.inode == after.inode
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/local_file_identity_matrix_tests.rs"]
mod matrix_tests;
