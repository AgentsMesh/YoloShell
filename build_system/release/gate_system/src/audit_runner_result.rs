use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::path::PathBuf;
use yolo_release_contracts::{AUDIT_RESULT_NAME, MAX_AUDIT_DIGEST_BYTES};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_foundation::file_seal::FileSeal;
use yolo_release_process_execution::constants::DESCRIPTOR_PATH_PREFIX;

const OVERFLOW_SENTINEL_BYTES: u64 = 1;
const SYSTEM_CALL_SUCCESS: libc::c_int = 0;
const INVALID_DESCRIPTOR: libc::c_int = -1;

pub(crate) fn descriptor_path(descriptor: libc::c_int) -> PathBuf {
    PathBuf::from(DESCRIPTOR_PATH_PREFIX).join(descriptor.to_string())
}

pub(crate) fn ensure_absent(directory: &DirectoryHandle) -> Result<()> {
    let name = CString::new(AUDIT_RESULT_NAME)?;
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    let status = unsafe {
        libc::fstatat(
            directory.raw_fd(),
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if status == SYSTEM_CALL_SUCCESS {
        ensure!(false, "bundle audit result already exists");
    }
    let error = std::io::Error::last_os_error();
    ensure!(
        error.raw_os_error() == Some(libc::ENOENT),
        "inspect bundle audit result: {error}"
    );
    Ok(())
}

pub(crate) fn read(directory: &DirectoryHandle) -> Result<String> {
    let name = CString::new(AUDIT_RESULT_NAME)?;
    let descriptor = unsafe {
        libc::openat(
            directory.raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor <= INVALID_DESCRIPTOR {
        return Err(std::io::Error::last_os_error()).context("open bundle audit result");
    }
    let file = unsafe { File::from_raw_fd(descriptor) };
    let seal = FileSeal::capture(&file).context("bundle audit result must be a file")?;
    ensure!(
        seal.length() <= MAX_AUDIT_DIGEST_BYTES,
        "bundle audit result is too large"
    );
    let mut bytes = Vec::new();
    (&file)
        .take(MAX_AUDIT_DIGEST_BYTES.saturating_add(OVERFLOW_SENTINEL_BYTES))
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_AUDIT_DIGEST_BYTES,
        "bundle audit result is too large"
    );
    seal.verify(&file, "bundle audit result changed while read")?;
    String::from_utf8(bytes).context("bundle audit result is not UTF-8")
}
