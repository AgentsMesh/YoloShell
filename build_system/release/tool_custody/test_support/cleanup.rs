use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use yolo_release_contracts::{NO_FILE_FLAGS, PRIVATE_DIRECTORY_MODE};
use yolo_release_filesystem::DirectoryHandle;

pub(super) const SENTINEL_NAME: &str = "foreign-sentinel";
pub(super) const SENTINEL_BYTES: &[u8] = b"unrelated retained replacement";
pub(super) const READ_ONLY_MODE: u32 = 0o400;
const MODE_MASK: u32 = 0o777;

pub(crate) fn rebind(private: &Path, moved: &Path) {
    let handle = DirectoryHandle::open(private).unwrap();
    handle.set_flags(NO_FILE_FLAGS).unwrap();
    handle.set_mode(PRIVATE_DIRECTORY_MODE).unwrap();
    fs::rename(private, moved).unwrap();
    fs::create_dir(private).unwrap();
    fs::write(private.join(SENTINEL_NAME), SENTINEL_BYTES).unwrap();
    fs::set_permissions(
        private.join(SENTINEL_NAME),
        fs::Permissions::from_mode(READ_ONLY_MODE),
    )
    .unwrap();
}

pub(crate) fn assert_preserved_and_remove(private: &Path) {
    let sentinel = private.join(SENTINEL_NAME);
    assert_eq!(fs::read(&sentinel).unwrap(), SENTINEL_BYTES);
    assert_eq!(
        fs::metadata(&sentinel).unwrap().permissions().mode() & MODE_MASK,
        READ_ONLY_MODE
    );
    fs::remove_file(sentinel).unwrap();
    fs::remove_dir(private).unwrap();
}

pub(crate) fn failed_capture_script(probe: &Path, moved: &Path) -> String {
    let text = std::str::from_utf8(SENTINEL_BYTES).unwrap();
    format!(
        "#!/bin/bash\nset -eu\nprivate=${{0%/*}}\nprintf '%s' \"$private\" > '{}'\n/usr/bin/chflags nouchg \"$private\"\n/bin/chmod {PRIVATE_DIRECTORY_MODE:o} \"$private\"\n/bin/mv \"$private\" '{}'\n/bin/mkdir \"$private\"\nprintf '%s' '{text}' > \"$private/{SENTINEL_NAME}\"\n/bin/chmod {READ_ONLY_MODE:o} \"$private/{SENTINEL_NAME}\"\nprintf 'wrong-version\\n'\n",
        probe.display(),
        moved.display()
    )
}
