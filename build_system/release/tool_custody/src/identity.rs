use yolo_release_foundation::digest::opened_file_sha256;
use yolo_release_filesystem::DirectoryHandle;
use anyhow::{Context, Result, ensure};
use std::fs::{File, OpenOptions};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

pub fn verify_private_path(
    root: &DirectoryHandle,
    path: &Path,
    pinned: &File,
    identity: (u64, u64),
    digest: &str,
) -> Result<()> {
    root.verify_path()?;
    let reopened = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .context("reopen private Bazel launcher")?;
    let metadata = reopened.metadata()?;
    ensure!(
        (metadata.dev(), metadata.ino()) == identity,
        "private Bazel launcher path identity changed"
    );
    ensure!(
        opened_file_sha256(&reopened)? == digest && opened_file_sha256(pinned)? == digest,
        "private Bazel launcher changed"
    );
    Ok(())
}
