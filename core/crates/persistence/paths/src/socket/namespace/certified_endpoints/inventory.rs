use super::super::super::{socket_path, socket_stage, stream};
use crate::paths::{
    constants::{
        MAX_PRIVATE_DIRECTORY_ENTRIES, PERMISSION_MODE_MASK, PRIVATE_FILE_MODE,
        SOCKET_ENDPOINT_LINK_COUNT,
    },
    directory::{PrivateDirectory, entry_c_string},
    identity::ObjectIdentity,
};
use anyhow::{Result, ensure};
use std::{
    ffi::{CStr, CString},
    mem::MaybeUninit,
};

const UNPROVEN: &str = "certified socket namespace contains an unproven entry";
pub(super) type Inventory = Vec<(CString, ObjectIdentity)>;

pub(super) fn inspect(directory: &PrivateDirectory) -> Result<Inventory> {
    directory.verify_path()?;
    let mut entries = stream::DirectoryStream::open(directory.descriptor())?;
    let mut inventory = Vec::new();
    let mut complete = false;
    for _ in 0..MAX_PRIVATE_DIRECTORY_ENTRIES {
        let Some(name) = entries.next_name()? else {
            complete = true;
            break;
        };
        let name = entry_c_string(&name)?;
        if socket_stage::is_lifecycle(&name) {
            continue;
        }
        ensure!(socket_path::is_endpoint_name(&name), UNPROVEN);
        inventory.push((name.clone(), identity(directory, &name)?));
    }
    ensure!(complete, UNPROVEN);
    inventory.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
    directory.verify_path()?;
    Ok(inventory)
}

pub(super) fn identity(directory: &PrivateDirectory, name: &CStr) -> Result<ObjectIdentity> {
    let mut metadata = MaybeUninit::<libc::stat>::uninit();
    // SAFETY: directory and name remain retained; fstatat writes one stat object.
    let result = unsafe {
        libc::fstatat(
            directory.descriptor(),
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != libc::c_int::default() {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: successful fstatat initialized every stat field.
    let metadata = unsafe { metadata.assume_init() };
    ensure!(
        metadata.st_mode & libc::S_IFMT == libc::S_IFSOCK
            && metadata.st_uid == unsafe { libc::geteuid() }
            && u64::from(metadata.st_nlink) == SOCKET_ENDPOINT_LINK_COUNT
            && u32::from(metadata.st_mode) & PERMISSION_MODE_MASK == PRIVATE_FILE_MODE,
        UNPROVEN
    );
    ObjectIdentity::inspect_stat(&metadata)
}
