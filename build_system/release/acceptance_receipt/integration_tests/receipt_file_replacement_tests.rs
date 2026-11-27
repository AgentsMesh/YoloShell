use crate::constants::{READ_ONLY_FILE_MODE, RECEIPT_FILE_ERROR, RECEIPT_FILE_TYPE_ERROR};
use crate::io::open_bounded;
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;

const RECEIPT: &str = "receipt.json";
const PARENT: &str = "current";
const RETIRED_PARENT: &str = "retained";
const CONTENT: &[u8] = b"{}";
const SUCCESS: libc::c_int = 0;

fn fifo(path: &Path) {
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe { libc::mkfifo(path.as_ptr(), READ_ONLY_FILE_MODE as libc::mode_t) },
        SUCCESS
    );
}

#[test]
fn unopened_fifo_receipt_is_rejected_without_waiting_for_a_writer() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(RECEIPT);
    fifo(&path);
    assert_eq!(
        open_bounded(&path).err().unwrap().to_string(),
        RECEIPT_FILE_TYPE_ERROR
    );
}

#[test]
fn parent_replacement_cannot_block_or_substitute_the_retained_receipt() {
    for replacement in [
        Replacement::Fifo,
        Replacement::Symlink,
        Replacement::File,
        Replacement::Missing,
    ] {
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join(PARENT);
        fs::create_dir(&parent).unwrap();
        let path = parent.join(RECEIPT);
        fs::write(&path, CONTENT).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(READ_ONLY_FILE_MODE)).unwrap();
        let receipt = open_bounded(&path).unwrap();
        assert_eq!(receipt.bytes(), CONTENT);
        receipt.verify_unchanged().unwrap();
        let moved = root.path().join(RETIRED_PARENT);
        fs::rename(&parent, &moved).unwrap();
        fs::create_dir(&parent).unwrap();
        match replacement {
            Replacement::Fifo => fifo(&path),
            Replacement::Symlink => symlink(moved.join(RECEIPT), &path).unwrap(),
            Replacement::File => {
                fs::write(&path, CONTENT).unwrap();
                fs::set_permissions(&path, fs::Permissions::from_mode(READ_ONLY_FILE_MODE))
                    .unwrap();
            }
            Replacement::Missing => {}
        }
        assert_eq!(
            receipt.verify_unchanged().unwrap_err().to_string(),
            RECEIPT_FILE_ERROR
        );
        assert_eq!(fs::read(moved.join(RECEIPT)).unwrap(), CONTENT);
    }
}

enum Replacement {
    Fifo,
    Symlink,
    File,
    Missing,
}
