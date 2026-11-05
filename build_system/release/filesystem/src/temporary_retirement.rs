use crate::DirectoryHandle;
use crate::constants::TEMPORARY_RANDOM_BYTES;
use anyhow::{Context, Result, anyhow, ensure};
use std::ffi::{CStr, CString};
use std::path::Path;
use yolo_release_foundation::digest::hex_digest;

const RETIREMENT_PREFIX: &str = ".yolo-release-retired-";
const SYSTEM_CALL_SUCCESS: libc::c_int = 0;
const UNLINK_FILE_FLAGS: libc::c_int = 0;
const RETIREMENT_ERROR: &str = "retire owned release temporary entry";
const REPLACEMENT_ERROR: &str = "release temporary cleanup encountered a replacement";
const RANDOM_ERROR: &str = "read secure release retirement randomness";

pub(super) fn random_name() -> Result<CString> {
    let mut bytes = [0_u8; TEMPORARY_RANDOM_BYTES];
    let status = unsafe { libc::getentropy(bytes.as_mut_ptr().cast(), bytes.len()) };
    if status != SYSTEM_CALL_SUCCESS {
        return Err(std::io::Error::last_os_error()).context(RANDOM_ERROR);
    }
    CString::new(format!("{RETIREMENT_PREFIX}{}", hex_digest(bytes))).map_err(Into::into)
}

pub(super) fn rename(parent: &DirectoryHandle, from: &CStr, to: &CStr) -> Result<()> {
    let status = unsafe {
        libc::renameatx_np(
            parent.raw_fd(),
            from.as_ptr(),
            parent.raw_fd(),
            to.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    if status != SYSTEM_CALL_SUCCESS {
        return Err(std::io::Error::last_os_error()).context(RETIREMENT_ERROR);
    }
    Ok(())
}

pub(super) fn confirm_or_restore(
    parent: &DirectoryHandle,
    original: &CStr,
    retired: &CStr,
    path: &Path,
    confirmation: Result<()>,
) -> Result<()> {
    if let Err(error) = confirmation {
        return match rename(parent, retired, original) {
            Ok(()) => Err(error).context(REPLACEMENT_ERROR),
            Err(restore) => Err(anyhow!(
                "{REPLACEMENT_ERROR}: {error:#}; preserve {}: {restore:#}",
                path.display()
            )),
        };
    }
    Ok(())
}

pub(super) fn unlink(parent: &DirectoryHandle, name: &CStr, directory: bool) -> Result<()> {
    let flags = if directory {
        libc::AT_REMOVEDIR
    } else {
        UNLINK_FILE_FLAGS
    };
    ensure!(
        unsafe { libc::unlinkat(parent.raw_fd(), name.as_ptr(), flags) } == SYSTEM_CALL_SUCCESS,
        "remove retired release temporary entry: {}",
        std::io::Error::last_os_error()
    );
    Ok(())
}
