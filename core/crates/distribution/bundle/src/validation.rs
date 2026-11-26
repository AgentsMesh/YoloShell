use crate::{BundleError, constants::*};

pub(crate) fn lowercase_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub(crate) fn digest(value: &str) -> bool {
    value.len() == SHA256_HEX_BYTES && lowercase_hex(value)
}

pub(crate) fn source_revision(value: &str) -> Result<(), BundleError> {
    if SOURCE_REVISION_HEX_BYTES.contains(&value.len()) && lowercase_hex(value) {
        Ok(())
    } else {
        Err(BundleError::SourceRevision)
    }
}

pub(crate) fn version(value: &str) -> Result<(), BundleError> {
    if !value.is_empty()
        && value.len() <= MAX_VERSION_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || VERSION_PUNCTUATION.contains(&byte))
    {
        Ok(())
    } else {
        Err(BundleError::Version)
    }
}
