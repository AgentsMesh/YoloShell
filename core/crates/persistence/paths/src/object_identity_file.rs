use super::ObjectIdentity;
use anyhow::Result;
use std::fs::File;

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(super) fn from_file(file: &File) -> Result<Option<ObjectIdentity>> {
    Ok(Some(
        super::file_identity_descriptor::inspect(file)?.object_identity(),
    ))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub(super) fn from_file(_file: &File) -> Result<Option<ObjectIdentity>> {
    Ok(None)
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/object_identity_file_inline_tests.rs"]
mod tests;
