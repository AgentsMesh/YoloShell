use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

pub(super) type Snapshot = Vec<(PathBuf, u64, u64, u32, i64, i64, i64, i64, Vec<u8>)>;

pub(super) fn snapshot(root: &Path) -> Snapshot {
    let mut entries = Vec::new();
    collect(root, root, &mut entries);
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    entries
}

fn collect(root: &Path, path: &Path, entries: &mut Snapshot) {
    let metadata = std::fs::symlink_metadata(path).unwrap();
    let bytes = if metadata.is_file() {
        std::fs::read(path).unwrap()
    } else if metadata.file_type().is_symlink() {
        std::fs::read_link(path)
            .unwrap()
            .as_os_str()
            .as_encoded_bytes()
            .to_vec()
    } else {
        Vec::new()
    };
    entries.push((
        path.strip_prefix(root).unwrap().to_path_buf(),
        metadata.dev(),
        metadata.ino(),
        metadata.mode(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
        bytes,
    ));
    if metadata.is_dir() {
        for entry in std::fs::read_dir(path).unwrap() {
            collect(root, &entry.unwrap().path(), entries);
        }
    }
}
