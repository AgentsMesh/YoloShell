use crate::constants::INVENTORY_MISMATCH;
use anyhow::{Result, ensure};
use std::{collections::BTreeSet, ffi::OsString};
use yolo_paths::{directory::PrivateDirectory, directory_stream::DirectoryStream};

pub(crate) fn exact(directory: &PrivateDirectory, names: &[&str]) -> Result<()> {
    directory.verify_path()?;
    let mut expected = names.iter().map(OsString::from).collect::<BTreeSet<_>>();
    let mut stream = DirectoryStream::open(directory.descriptor())?;
    while let Some(name) = stream.next_name()? {
        ensure!(expected.remove(&name), INVENTORY_MISMATCH);
    }
    ensure!(expected.is_empty(), INVENTORY_MISMATCH);
    directory.verify_path()
}

pub(crate) fn exists(directory: &PrivateDirectory, name: &str) -> Result<bool> {
    directory.verify_path()?;
    let name = std::ffi::CString::new(name)?;
    let mut status = std::mem::MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            directory.descriptor(),
            name.as_ptr(),
            status.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    let observed = if result == 0 {
        Ok(true)
    } else {
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::NotFound {
            Ok(false)
        } else {
            Err(error)
        }
    };
    directory.verify_path()?;
    observed.map_err(Into::into)
}
