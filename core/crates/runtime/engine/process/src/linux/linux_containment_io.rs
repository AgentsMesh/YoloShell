use super::LinuxContainmentError;
use super::constants::*;
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::FileExt;

const RETIREMENT_PROBE_BYTES: usize = 1;

pub(super) fn retained_control_was_removed(control: &File) -> Result<bool, LinuxContainmentError> {
    // kernfs returns ENODEV only after this retained cgroup control is deactivated.
    let mut buffer = [u8::default(); RETIREMENT_PROBE_BYTES];
    match control.read_at(&mut buffer, u64::default()) {
        Err(error) if error.raw_os_error() == Some(libc::ENODEV) => Ok(true),
        Err(error) => Err(error.into()),
        Ok(_) => Ok(false),
    }
}

pub(super) fn read_control(directory: &File, name: &str) -> Result<String, LinuxContainmentError> {
    let mut file = open_control(directory, name, libc::O_RDONLY)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_CONTROL_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_CONTROL_BYTES {
        return Err(LinuxContainmentError::Identity(
            "cgroup control data is oversized",
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| LinuxContainmentError::Identity("cgroup control data is not UTF-8"))
}

pub(super) fn write_control(
    directory: &File,
    name: &str,
    value: &str,
) -> Result<(), LinuxContainmentError> {
    let mut file = open_control(directory, name, libc::O_WRONLY)?;
    file.write_all(value.as_bytes())?;
    Ok(())
}

pub(super) fn contains_word(
    directory: &File,
    name: &str,
    expected: &str,
) -> Result<bool, LinuxContainmentError> {
    Ok(read_control(directory, name)?
        .split_ascii_whitespace()
        .any(|word| word == expected))
}

pub(super) fn open_control(
    directory: &File,
    name: &str,
    access: libc::c_int,
) -> Result<File, LinuxContainmentError> {
    super::descriptor::validate_leaf_name(name)?;
    let name = CString::new(name)
        .map_err(|_| LinuxContainmentError::Identity("control name contains a null byte"))?;
    let flags = access | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags,
            OPEN_MODE_UNUSED,
        )
    };
    if descriptor == INVALID_DESCRIPTOR {
        Err(std::io::Error::last_os_error().into())
    } else {
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}
