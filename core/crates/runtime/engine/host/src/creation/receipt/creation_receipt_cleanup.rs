use super::*;
use std::os::unix::ffi::OsStrExt;
use std::os::fd::AsRawFd;

pub(crate) fn cleanup_stale_receipts(index: &ReceiptDirectory) -> Result<()> {
    cleanup_stale_receipts_with(index, |_| Ok(()))
}

pub(crate) fn cleanup_stale_receipts_with<F>(index: &ReceiptDirectory, mut hook: F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    index.verify_path("creation receipt staging")?;
    let mut stream = ReceiptDirectoryStream::open(index.file.as_raw_fd())?;
    let mut scanned = 0_usize;
    while let Some(name) = stream.next_name()? {
        scanned += 1;
        anyhow::ensure!(
            scanned <= MAX_CREATION_RECEIPT_STAGING_ENTRIES,
            "creation receipt staging entry limit exceeded"
        );
        if !stale_receipt_name(name.as_os_str().as_bytes()) {
            continue;
        }
        let name = std::ffi::CString::new(name.as_os_str().as_bytes())?;
        let opened =
            open_receipt_staging(index, &name)?.context("stale creation receipt disappeared")?;
        let mut staged = ReceiptStagingFile {
            file: opened.0,
            _lock: opened.1,
            identity: opened.2,
            slot: receipt_slot_name(&name)?,
            name,
        };
        retire_receipt_staging_with(index, &mut staged, &mut |stage, path| {
            if stage == ReceiptRetirementStage::BeforeMove {
                hook(path)
            } else {
                Ok(())
            }
        })?;
    }
    index.verify_path("creation receipt staging")
}

pub(crate) fn retire_receipt_staging(
    staging: &ReceiptDirectory,
    staged: &mut ReceiptStagingFile,
) -> Result<()> {
    retire_receipt_staging_with(staging, staged, &mut |_, _| Ok(()))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReceiptRetirementStage {
    BeforeMove,
    AfterMove,
    AfterDirectorySync,
    AfterTruncate,
}

pub(crate) fn retire_receipt_staging_with<F>(
    staging: &ReceiptDirectory,
    staged: &mut ReceiptStagingFile,
    hook: &mut F,
) -> Result<()>
where
    F: FnMut(ReceiptRetirementStage, &Path) -> Result<()>,
{
    use std::os::unix::fs::MetadataExt;
    if receipt_name_is_missing(staging, &staged.name)? {
        return Ok(());
    }
    let path = receipt_entry_path(staging, &staged.name);
    let mode = staged.file.metadata()?.mode() & FILE_MODE_MASK;
    verify_receipt_file(staged.identity, &staged.file, &path, mode)?;
    verify_receipt_named(staging, &staged.name, staged.identity, mode)?;
    anyhow::ensure!(
        receipt_name_is_missing(staging, &staged.slot)?,
        "receipt staging retirement slot is occupied"
    );
    hook(ReceiptRetirementStage::BeforeMove, &path)?;
    let moved = move_receipt_exclusive(
        staging.file.as_raw_fd(),
        &staged.name,
        staging.file.as_raw_fd(),
        &staged.slot,
    )?;
    anyhow::ensure!(moved, "receipt staging retirement slot appeared");
    if let Err(error) = hook(ReceiptRetirementStage::AfterMove, &path) {
        return rollback_receipt_retirement(staging, staged, error);
    }
    if let Err(error) = verify_receipt_named(staging, &staged.slot, staged.identity, mode) {
        return rollback_receipt_retirement(staging, staged, error);
    }
    staging.file.sync_all()?;
    if let Err(error) = hook(ReceiptRetirementStage::AfterDirectorySync, &path) {
        return rollback_receipt_retirement(staging, staged, error);
    }
    if let Err(error) = verify_receipt_named(staging, &staged.slot, staged.identity, mode) {
        return rollback_receipt_retirement(staging, staged, error);
    }
    set_receipt_mode(&staged.file, &path, CREATION_RECEIPT_TEMP_MODE)?;
    staged.file.set_len(0)?;
    staged.file.sync_all()?;
    staged.identity = inspect_receipt_file(&staged.file, &path, CREATION_RECEIPT_TEMP_MODE)?;
    if let Err(error) = hook(ReceiptRetirementStage::AfterTruncate, &path) {
        return rollback_receipt_retirement(staging, staged, error);
    }
    if let Err(error) = verify_receipt_named(
        staging,
        &staged.slot,
        staged.identity,
        CREATION_RECEIPT_TEMP_MODE,
    ) {
        return rollback_receipt_retirement(staging, staged, error);
    }
    staging.verify_path("creation receipt staging")
}

pub(crate) fn rollback_receipt_retirement(
    staging: &ReceiptDirectory,
    staged: &ReceiptStagingFile,
    cause: anyhow::Error,
) -> Result<()> {
    let restored = move_receipt_exclusive(
        staging.file.as_raw_fd(),
        &staged.slot,
        staging.file.as_raw_fd(),
        &staged.name,
    );
    staging.file.sync_all()?;
    match restored {
        Ok(true) => Err(cause.context("receipt staging retirement was rolled back")),
        Ok(false) => Err(cause.context("receipt staging replacement was retained in its slot")),
        Err(error) => Err(cause.context(format!(
            "receipt staging rollback failed with both names retained: {error:#}"
        ))),
    }
}

pub(crate) struct ReceiptDirectoryStream(*mut libc::DIR);

impl ReceiptDirectoryStream {
    pub(crate) fn open(descriptor: libc::c_int) -> Result<Self> {
        let current = std::ffi::CString::new(CURRENT_DIRECTORY_ENTRY_BYTES)?;
        let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
        let opened = unsafe { libc::openat(descriptor, current.as_ptr(), flags) };
        if opened < 0 {
            return Err(std::io::Error::last_os_error()).context("open receipt staging stream");
        }
        let stream = unsafe { receipt_fdopendir(opened) };
        if stream.is_null() {
            let error = std::io::Error::last_os_error();
            unsafe { libc::close(opened) };
            return Err(error).context("enumerate receipt staging");
        }
        Ok(Self(stream))
    }

    pub(crate) fn next_name(&mut self) -> Result<Option<std::ffi::OsString>> {
        use std::os::unix::ffi::OsStrExt;
        unsafe { *receipt_errno_location() = 0 };
        let entry = unsafe { receipt_readdir(self.0) };
        if entry.is_null() {
            let code = unsafe { *receipt_errno_location() };
            return if code == 0 {
                Ok(None)
            } else {
                Err(std::io::Error::from_raw_os_error(code)).context("read receipt staging")
            };
        }
        let bytes = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if bytes == CURRENT_DIRECTORY_ENTRY_BYTES || bytes == PARENT_DIRECTORY_ENTRY_BYTES {
            return self.next_name();
        }
        Ok(Some(std::ffi::OsStr::from_bytes(bytes).to_owned()))
    }
}

impl Drop for ReceiptDirectoryStream {
    fn drop(&mut self) {
        unsafe { libc::closedir(self.0) };
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn receipt_errno_location() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

#[cfg(target_os = "linux")]
pub(crate) fn receipt_errno_location() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}

include!("creation_receipt_cleanup_stream_io.rs");
