use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReceiptReadStage {
    MissingObserved,
    AfterRead,
}

pub(crate) fn read_receipt_at(
    index: &ReceiptDirectory,
    name: &str,
) -> Result<Option<CreationReceipt>> {
    read_receipt_at_with(index, name, |_| Ok(()))
}

pub(crate) fn read_receipt_at_with<F>(
    index: &ReceiptDirectory,
    name: &str,
    mut hook: F,
) -> Result<Option<CreationReceipt>>
where
    F: FnMut(ReceiptReadStage) -> Result<()>,
{
    use std::io::Read;
    let name = std::ffi::CString::new(name).context("receipt index name contains a null byte")?;
    let path = receipt_entry_path(index, &name);
    let Some(mut file) = open_receipt_file_confirmed(index, &name, &path, || {
        hook(ReceiptReadStage::MissingObserved)
    })?
    else {
        return Ok(None);
    };
    let identity = inspect_receipt_file(&file, &path, CREATION_RECEIPT_FILE_MODE)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_CREATION_RECEIPT_BYTES + SIZE_LIMIT_SENTINEL_BYTES)
        .read_to_end(&mut bytes)?;
    anyhow::ensure!(
        bytes.len() as u64 <= MAX_CREATION_RECEIPT_BYTES,
        "receipt exceeds byte limit: {}",
        path.display()
    );
    hook(ReceiptReadStage::AfterRead)?;
    verify_receipt_file(identity, &file, &path, CREATION_RECEIPT_FILE_MODE)?;
    let current = open_receipt_file_confirmed(index, &name, &path, || Ok(()))?
        .context("published creation receipt disappeared")?;
    let current_identity = inspect_receipt_file(&current, &path, CREATION_RECEIPT_FILE_MODE)?;
    anyhow::ensure!(
        identity == current_identity,
        "creation receipt path changed"
    );
    let receipt: CreationReceipt = serde_json::from_slice(&bytes)
        .with_context(|| format!("parse creation receipt {}", path.display()))?;
    receipt.validate()?;
    Ok(Some(receipt))
}

pub(crate) fn open_receipt_file_confirmed<F>(
    index: &ReceiptDirectory,
    name: &std::ffi::CStr,
    path: &Path,
    mut after_missing: F,
) -> Result<Option<File>>
where
    F: FnMut() -> Result<()>,
{
    use std::os::fd::AsRawFd;
    index.verify_path(CREATION_RECEIPT_DIRECTORY_LABEL)?;
    let opened = open_receipt_file_at(
        index.file.as_raw_fd(),
        name,
        path,
        CREATION_RECEIPT_LABEL,
        libc::O_RDONLY,
    )?;
    if opened.is_some() {
        index.verify_path(CREATION_RECEIPT_DIRECTORY_LABEL)?;
        return Ok(opened);
    }
    after_missing()?;
    index.verify_path(CREATION_RECEIPT_DIRECTORY_LABEL)?;
    let confirmed = open_receipt_file_at(
        index.file.as_raw_fd(),
        name,
        path,
        CREATION_RECEIPT_LABEL,
        libc::O_RDONLY,
    )?;
    index.verify_path(CREATION_RECEIPT_DIRECTORY_LABEL)?;
    Ok(confirmed)
}

#[cfg(test)]
pub(crate) fn read_receipt_at_with_test_hook<F>(
    index: &ReceiptDirectory,
    name: &str,
    hook: F,
) -> Result<Option<CreationReceipt>>
where
    F: FnMut(ReceiptReadStage) -> Result<()>,
{
    read_receipt_at_with(index, name, hook)
}
