use crate::constants::*;
use crate::constants::{EXPECTED_FILE_LINKS, PERMISSION_BITS_MASK, READ_ONLY_FILE_MODE};
use anyhow::{Result, ensure};
use std::fs::{File, Metadata};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use yolo_release_filesystem::ensure_no_extended_acl;

pub(crate) fn validate_descriptor(file: &File, metadata: &Metadata) -> Result<()> {
    ensure!(metadata.is_file(), RECEIPT_FILE_TYPE_ERROR);
    ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        RECEIPT_FILE_OWNER_ERROR
    );
    ensure!(
        metadata.mode() & PERMISSION_BITS_MASK == READ_ONLY_FILE_MODE,
        RECEIPT_FILE_MODE_ERROR
    );
    ensure!(
        metadata.nlink() == EXPECTED_FILE_LINKS,
        RECEIPT_FILE_LINK_ERROR
    );
    ensure_no_extended_acl(file.as_raw_fd(), RECEIPT_FILE_ACL_ERROR)
}

pub(crate) fn stable(before: &Metadata, after: &Metadata) -> bool {
    before.dev() == after.dev()
        && before.ino() == after.ino()
        && before.uid() == after.uid()
        && before.mode() == after.mode()
        && before.nlink() == after.nlink()
        && before.len() == after.len()
        && before.mtime() == after.mtime()
        && before.mtime_nsec() == after.mtime_nsec()
        && before.ctime() == after.ctime()
        && before.ctime_nsec() == after.ctime_nsec()
}
