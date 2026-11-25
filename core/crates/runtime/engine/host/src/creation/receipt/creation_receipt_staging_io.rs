use super::*;

pub(crate) fn set_receipt_mode(file: &File, path: &Path, mode: u32) -> Result<()> {
    use std::os::fd::AsRawFd;
    let result = unsafe { libc::fchmod(file.as_raw_fd(), mode as libc::mode_t) };
    if result != 0 {
        Err(std::io::Error::last_os_error()).context("set receipt staging permissions")
    } else {
        crate::private_acl::clear(file, path)
    }
}

pub(crate) fn lock_receipt_file(
    file: &File,
) -> Result<crate::paths::process_file_lock::ProcessFileLock> {
    let identity = crate::paths::identity::ObjectIdentity::from_file(file)?
        .context("receipt staging birth identity is unavailable")?;
    crate::paths::process_file_lock::ProcessFileLock::try_lock(
        file,
        identity,
        crate::paths::process_file_lock::Mode::Exclusive,
    )?
    .context("lock receipt staging file")
}

pub(crate) fn verify_receipt_named(
    directory: &ReceiptDirectory,
    name: &std::ffi::CStr,
    expected: ReceiptFileIdentity,
    mode: u32,
) -> Result<()> {
    use std::os::fd::AsRawFd;
    let path = receipt_entry_path(directory, name);
    let current = open_receipt_file_at(
        directory.file.as_raw_fd(),
        name,
        &path,
        CREATION_RECEIPT_LABEL,
        libc::O_RDONLY,
    )?
    .context("creation receipt entry disappeared")?;
    let identity = inspect_receipt_file(&current, &path, mode)?;
    anyhow::ensure!(
        identity.same_content(expected),
        "creation receipt entry changed"
    );
    directory.verify_path(CREATION_RECEIPT_DIRECTORY_LABEL)
}

pub(crate) fn receipt_name_is_missing(
    directory: &ReceiptDirectory,
    name: &std::ffi::CStr,
) -> Result<bool> {
    let path = receipt_entry_path(directory, name);
    open_receipt_file_confirmed(directory, name, &path, || Ok(())).map(|opened| opened.is_none())
}

pub(crate) fn move_receipt_exclusive(
    source_directory: libc::c_int,
    source: &std::ffi::CStr,
    target_directory: libc::c_int,
    target: &std::ffi::CStr,
) -> Result<bool> {
    move_receipt_exclusive_with_fence(
        source_directory,
        source,
        target_directory,
        target,
        || Ok(()),
    )
}

pub(crate) fn move_receipt_exclusive_with_fence<F>(
    source_directory: libc::c_int,
    source: &std::ffi::CStr,
    target_directory: libc::c_int,
    target: &std::ffi::CStr,
    commit_fence: F,
) -> Result<bool>
where
    F: FnOnce() -> Result<()>,
{
    use crate::paths::private_file::rename_exclusive_between;
    commit_fence()?;
    match rename_exclusive_between(source_directory, source, target_directory, target) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(error).context("move creation receipt exclusively"),
    }
}
