use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs::{File, Metadata};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use yolo_release_archive_audit::constants::{PATH_SEPARATOR, PATH_SEPARATOR_LENGTH};
use yolo_release_archive_audit::model::ZipInventory;
use yolo_release_contracts::*;
use yolo_release_contracts::{EXPECTED_FILE_LINKS, PERMISSION_BITS_MASK};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_filesystem::ensure_no_extended_acl;
use yolo_release_foundation::digest::opened_file_sha256;
use yolo_release_gate_system::entry_names;

#[derive(Clone, Debug, Eq, PartialEq)]
struct TreeEntry {
    directory: bool,
    mode: u16,
    size: u64,
    sha256: Option<String>,
}

pub fn verify_tree(container: &DirectoryHandle, inventory: &ZipInventory) -> Result<()> {
    let expected = expected_tree(inventory)?;
    let actual = capture_tree(container)?;
    ensure!(actual == expected, DELIVERY_TREE_MISMATCH_ERROR);
    container.verify_self()
}

fn expected_tree(inventory: &ZipInventory) -> Result<BTreeMap<Vec<u8>, TreeEntry>> {
    let mut result = BTreeMap::new();
    for entry in &inventory.entries {
        let mut path = entry.path.as_bytes().to_vec();
        if entry.is_directory {
            ensure!(
                path.pop() == Some(PATH_SEPARATOR),
                DELIVERY_TREE_MISMATCH_ERROR
            );
        }
        ensure!(
            result
                .insert(
                    path,
                    TreeEntry {
                        directory: entry.is_directory,
                        mode: entry.mode,
                        size: entry.uncompressed_size,
                        sha256: (!entry.is_directory).then(|| entry.sha256.clone()),
                    },
                )
                .is_none(),
            DELIVERY_TREE_MISMATCH_ERROR
        );
    }
    Ok(result)
}

fn capture_tree(container: &DirectoryHandle) -> Result<BTreeMap<Vec<u8>, TreeEntry>> {
    container.verify_self()?;
    let root = DirectoryHandle::open_at(container, APP_NAME)?;
    let mut result = BTreeMap::new();
    record_directory(APP_NAME.as_bytes().to_vec(), &root, &mut result)?;
    Ok(result)
}

fn record_directory(
    path: Vec<u8>,
    directory: &DirectoryHandle,
    result: &mut BTreeMap<Vec<u8>, TreeEntry>,
) -> Result<()> {
    let metadata = directory.retained_file().metadata()?;
    validate_owner(&metadata)?;
    ensure_no_extended_acl(directory.raw_fd(), DELIVERY_TREE_ENTRY_ERROR)?;
    insert(result, path.clone(), true, &metadata, None)?;
    for name in entry_names(directory)? {
        let child_path = joined(&path, &name);
        let file = open_entry(directory, &name)?;
        let metadata = file.metadata()?;
        validate_owner(&metadata)?;
        ensure_no_extended_acl(file.as_raw_fd(), DELIVERY_TREE_ENTRY_ERROR)?;
        if metadata.is_dir() {
            let child = DirectoryHandle::open_at_bytes(directory, &name)?;
            let identity = child.identity();
            ensure!(
                identity.device == metadata.dev() && identity.inode == metadata.ino(),
                DELIVERY_TREE_MISMATCH_ERROR
            );
            record_directory(child_path, &child, result)?;
        } else {
            ensure!(metadata.is_file(), DELIVERY_TREE_TYPE_ERROR);
            ensure!(
                metadata.nlink() == EXPECTED_FILE_LINKS,
                DELIVERY_TREE_LINK_ERROR
            );
            insert(
                result,
                child_path,
                false,
                &metadata,
                Some(opened_file_sha256(&file)?),
            )?;
        }
    }
    directory.verify_self()
}

fn insert(
    result: &mut BTreeMap<Vec<u8>, TreeEntry>,
    path: Vec<u8>,
    directory: bool,
    metadata: &Metadata,
    sha256: Option<String>,
) -> Result<()> {
    let mode = u16::try_from(metadata.permissions().mode() & PERMISSION_BITS_MASK)?;
    let size = if directory { 0 } else { metadata.len() };
    ensure!(
        result
            .insert(
                path,
                TreeEntry {
                    directory,
                    mode,
                    size,
                    sha256,
                }
            )
            .is_none(),
        DELIVERY_TREE_MISMATCH_ERROR
    );
    Ok(())
}

fn validate_owner(metadata: &Metadata) -> Result<()> {
    ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        DELIVERY_TREE_OWNER_ERROR
    );
    Ok(())
}

fn joined(parent: &[u8], name: &[u8]) -> Vec<u8> {
    let mut path = Vec::with_capacity(parent.len() + name.len() + PATH_SEPARATOR_LENGTH);
    path.extend_from_slice(parent);
    path.push(PATH_SEPARATOR);
    path.extend_from_slice(name);
    path
}

fn open_entry(parent: &DirectoryHandle, name: &[u8]) -> Result<File> {
    let name = CString::new(name)?;
    let descriptor = unsafe {
        libc::openat(
            parent.raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor == DELIVERY_OPEN_FAILURE {
        return Err(std::io::Error::last_os_error()).context(DELIVERY_TREE_ENTRY_ERROR);
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}
