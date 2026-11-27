use crate::DirectoryHandle;
use crate::constants::{TEMPORARY_DIRECTORY_MODE, TEMPORARY_NO_FLAGS};
use crate::temporary_retirement::{confirm_or_restore, random_name, rename, unlink};
use crate::tree_manifest::sorted_names;
use anyhow::{Context, Result, ensure};
use std::ffi::{CStr, CString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const STAT_SUCCESS: libc::c_int = 0;
const ENTRY_CHANGED: &str = "release temporary entry changed before retirement";

pub(super) fn clear(
    root: &DirectoryHandle,
    root_path: &Path,
    children: &[(String, DirectoryHandle)],
) -> Result<()> {
    for (name, expected) in children {
        root.verify_child(name, expected)?;
    }
    root.set_flags(TEMPORARY_NO_FLAGS)?;
    root.set_mode(TEMPORARY_DIRECTORY_MODE)?;
    for name in sorted_names(root)? {
        let expected = children
            .iter()
            .find(|(known, _)| known.as_bytes() == name.as_bytes())
            .map(|(_, handle)| handle);
        remove_entry(root, root_path, &CString::new(name.as_bytes())?, expected)?;
    }
    Ok(())
}

fn remove_entry(
    parent: &DirectoryHandle,
    parent_path: &Path,
    name: &CStr,
    retained: Option<&DirectoryHandle>,
) -> Result<()> {
    let expected = metadata(parent, name)?;
    let directory = expected.st_mode & libc::S_IFMT == libc::S_IFDIR;
    let child = if directory {
        let child = DirectoryHandle::open_at_bytes(parent, name.to_bytes())?;
        ensure!(
            child.identity().device == expected.st_dev as u64
                && child.identity().inode == expected.st_ino,
            ENTRY_CHANGED
        );
        if let Some(retained) = retained {
            ensure!(child.identity() == retained.identity(), ENTRY_CHANGED);
        }
        Some(child)
    } else {
        ensure!(retained.is_none(), ENTRY_CHANGED);
        None
    };
    let retired = random_name()?;
    let retired_path = parent_path.join(std::ffi::OsStr::from_bytes(retired.to_bytes()));
    rename(parent, name, &retired)?;
    let confirmation = metadata(parent, &retired).and_then(|actual| {
        ensure!(same_object(&expected, &actual), ENTRY_CHANGED);
        Ok(())
    });
    confirm_or_restore(parent, name, &retired, &retired_path, confirmation)?;
    if let Some(child) = child {
        clear(&child, &retired_path, &[])?;
    }
    unlink(parent, &retired, directory)
}

fn metadata(parent: &DirectoryHandle, name: &CStr) -> Result<libc::stat> {
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent.raw_fd(),
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != STAT_SUCCESS {
        return Err(std::io::Error::last_os_error()).context("inspect release temporary entry");
    }
    Ok(unsafe { metadata.assume_init() })
}

fn same_object(left: &libc::stat, right: &libc::stat) -> bool {
    left.st_dev == right.st_dev
        && left.st_ino == right.st_ino
        && (left.st_mode & libc::S_IFMT) == (right.st_mode & libc::S_IFMT)
        && left.st_birthtime == right.st_birthtime
        && left.st_birthtime_nsec == right.st_birthtime_nsec
}
