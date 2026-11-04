use super::*;

pub(crate) struct ReceiptStagingFile {
    pub(crate) file: File,
    pub(crate) _lock: ReceiptStagingLock,
    pub(crate) identity: ReceiptFileIdentity,
    pub(crate) name: std::ffi::CString,
    pub(crate) slot: std::ffi::CString,
}

pub(crate) struct ReceiptStagingLock {
    pub(crate) _process_lock: crate::paths::process_file_lock::ProcessFileLock,
    pub(crate) _file: File,
}

pub(crate) fn acquire_receipt_staging(
    staging: &ReceiptDirectory,
    index: &ReceiptDirectory,
    target: &std::ffi::CStr,
) -> Result<ReceiptStagingFile> {
    staging.verify_path("creation receipt staging")?;
    let name = receipt_temporary_name(index, target)?;
    let slot = receipt_slot_name(&name)?;
    let path = receipt_entry_path(staging, &name);
    let (file, lock, identity) = if let Some(opened) = open_receipt_staging(staging, &name)? {
        opened
    } else if let Some(opened) = open_receipt_staging(staging, &slot)? {
        move_receipt_staging_slot(staging, &slot, &name, opened)?
    } else {
        create_receipt_staging(staging, &name, &path)?
    };
    Ok(ReceiptStagingFile {
        file,
        _lock: lock,
        identity,
        name,
        slot,
    })
}

pub(crate) fn open_receipt_staging(
    staging: &ReceiptDirectory,
    name: &std::ffi::CStr,
) -> Result<Option<(File, ReceiptStagingLock, ReceiptFileIdentity)>> {
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::MetadataExt;
    let path = receipt_entry_path(staging, name);
    let Some(lock) = open_receipt_file_at(
        staging.file.as_raw_fd(),
        name,
        &path,
        "creation receipt staging",
        libc::O_RDONLY,
    )?
    else {
        return Ok(None);
    };
    let metadata = lock.metadata()?;
    let mode = metadata.mode() & FILE_MODE_MASK;
    anyhow::ensure!(
        mode == CREATION_RECEIPT_TEMP_MODE || mode == CREATION_RECEIPT_FILE_MODE,
        "receipt staging file has unsafe permissions: {}",
        path.display()
    );
    crate::private_acl::require_empty(&lock, &path)?;
    let expected = ReceiptFileIdentity::inspect(&metadata, &path, mode)?;
    let process_lock = lock_receipt_file(&lock)?;
    set_receipt_mode(&lock, &path, CREATION_RECEIPT_TEMP_MODE)?;
    #[cfg(test)]
    creation_receipt_staging_support_tests::run(
        creation_receipt_staging_support_tests::Stage::AfterReadOpen,
        &path,
    )?;
    let file = open_receipt_file_at(
        staging.file.as_raw_fd(),
        name,
        &path,
        "creation receipt staging",
        libc::O_RDWR,
    )?
    .context("receipt staging file disappeared")?;
    let identity = inspect_receipt_file(&file, &path, CREATION_RECEIPT_TEMP_MODE)?;
    anyhow::ensure!(
        expected.same_content(identity),
        "receipt staging identity changed"
    );
    verify_receipt_named(staging, name, identity, CREATION_RECEIPT_TEMP_MODE)?;
    Ok(Some((
        file,
        ReceiptStagingLock {
            _file: lock,
            _process_lock: process_lock,
        },
        identity,
    )))
}

pub(crate) fn create_receipt_staging(
    staging: &ReceiptDirectory,
    name: &std::ffi::CStr,
    path: &Path,
) -> Result<(File, ReceiptStagingLock, ReceiptFileIdentity)> {
    use std::os::fd::{AsRawFd, FromRawFd};
    let flags = libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    #[cfg(test)]
    creation_receipt_staging_support_tests::run(
        creation_receipt_staging_support_tests::Stage::BeforeCreate,
        path,
    )?;
    let descriptor = unsafe {
        libc::openat(
            staging.file.as_raw_fd(),
            name.as_ptr(),
            flags,
            CREATION_RECEIPT_TEMP_MODE,
        )
    };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context("create receipt staging file");
    }
    let file = unsafe { File::from_raw_fd(descriptor) };
    set_receipt_mode(&file, path, CREATION_RECEIPT_TEMP_MODE)?;
    let lock = file.try_clone()?;
    let process_lock = lock_receipt_file(&lock)?;
    file.sync_all()?;
    staging.file.sync_all()?;
    let identity = inspect_receipt_file(&file, path, CREATION_RECEIPT_TEMP_MODE)?;
    verify_receipt_named(staging, name, identity, CREATION_RECEIPT_TEMP_MODE)?;
    Ok((
        file,
        ReceiptStagingLock {
            _file: lock,
            _process_lock: process_lock,
        },
        identity,
    ))
}

pub(crate) fn move_receipt_staging_slot(
    staging: &ReceiptDirectory,
    slot: &std::ffi::CStr,
    active: &std::ffi::CStr,
    opened: (File, ReceiptStagingLock, ReceiptFileIdentity),
) -> Result<(File, ReceiptStagingLock, ReceiptFileIdentity)> {
    use std::os::fd::AsRawFd;
    #[cfg(test)]
    creation_receipt_staging_support_tests::run(
        creation_receipt_staging_support_tests::Stage::BeforeSlotMove,
        &receipt_entry_path(staging, active),
    )?;
    anyhow::ensure!(
        move_receipt_exclusive(
            staging.file.as_raw_fd(),
            slot,
            staging.file.as_raw_fd(),
            active,
        )?,
        "receipt staging active name appeared"
    );
    verify_receipt_named(staging, active, opened.2, CREATION_RECEIPT_TEMP_MODE)?;
    staging.file.sync_all()?;
    Ok(opened)
}

pub(crate) fn write_receipt_staging(
    staging: &ReceiptDirectory,
    staged: &mut ReceiptStagingFile,
    encoded: &[u8],
) -> Result<()> {
    use std::io::{Seek, Write};
    staged.file.set_len(0)?;
    staged.file.rewind()?;
    staged.file.write_all(encoded)?;
    let path = receipt_entry_path(staging, &staged.name);
    set_receipt_mode(&staged.file, &path, CREATION_RECEIPT_FILE_MODE)?;
    staged.file.sync_all()?;
    staged.identity = inspect_receipt_file(&staged.file, &path, CREATION_RECEIPT_FILE_MODE)?;
    verify_receipt_named(
        staging,
        &staged.name,
        staged.identity,
        CREATION_RECEIPT_FILE_MODE,
    )
}
