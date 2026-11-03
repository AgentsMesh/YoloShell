use super::*;
use std::os::unix::ffi::OsStrExt;

pub(crate) fn receipt_temporary_name(
    index: &ReceiptDirectory,
    target: &std::ffi::CStr,
) -> Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt;
    let directory = index.path.as_os_str().as_bytes();
    let mut material = Vec::with_capacity(directory.len() + target.to_bytes().len());
    material.extend_from_slice(&directory.len().to_be_bytes());
    material.extend_from_slice(directory);
    material.extend_from_slice(target.to_bytes());
    let digest = russh::keys::HashAlg::Sha256.digest(&material);
    let identifier = Uuid::from_slice(&digest[..CREATION_RECEIPT_NAME_HASH_BYTES])?;
    std::ffi::CString::new(format!(
        "{CREATION_RECEIPT_TEMP_PREFIX}{identifier}{CREATION_RECEIPT_TEMP_SUFFIX}"
    ))
    .context("temporary receipt name contains a null byte")
}

pub(crate) fn receipt_slot_name(active: &std::ffi::CStr) -> Result<std::ffi::CString> {
    let identifier = receipt_temporary_identifier(active.to_bytes())
        .context("receipt staging name is not canonical")?;
    std::ffi::CString::new(format!("{CREATION_RECEIPT_SLOT_PREFIX}{identifier}"))
        .context("receipt staging slot name contains a null byte")
}

pub(crate) fn stale_receipt_name(name: &[u8]) -> bool {
    receipt_temporary_identifier(name).is_some()
}

pub(crate) fn receipt_temporary_identifier(name: &[u8]) -> Option<Uuid> {
    let value = name
        .strip_prefix(CREATION_RECEIPT_TEMP_PREFIX.as_bytes())?
        .strip_suffix(CREATION_RECEIPT_TEMP_SUFFIX.as_bytes())?;
    let value = std::str::from_utf8(value).ok()?;
    Uuid::parse_str(value)
        .ok()
        .filter(|identifier| identifier.hyphenated().to_string() == value)
}

pub(crate) fn receipt_entry_path(directory: &ReceiptDirectory, name: &std::ffi::CStr) -> PathBuf {
    directory
        .path
        .join(std::ffi::OsStr::from_bytes(name.to_bytes()))
}
