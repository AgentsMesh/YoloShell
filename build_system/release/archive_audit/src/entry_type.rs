use anyhow::{Result, ensure};

use crate::constants::*;

pub(crate) struct ValidatedType {
    pub mode: u16,
    pub is_directory: bool,
}

pub(crate) struct TypeFields {
    pub external_attributes: u32,
    pub has_directory_suffix: bool,
    pub method: u16,
    pub crc32: u32,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
}

pub(crate) fn validate(fields: TypeFields) -> Result<ValidatedType> {
    let unix_mode = (fields.external_attributes >> u16::BITS) as u16;
    let dos_attributes = (fields.external_attributes & DOS_ATTRIBUTE_MASK) as u16;
    let file_type = unix_mode & UNIX_TYPE_MASK;
    let mode = unix_mode & UNIX_PERMISSION_MASK;
    ensure!(
        mode & UNIX_SPECIAL_MODE_MASK == NO_SPECIAL_MODE,
        "ZIP entry has setuid, setgid, or sticky mode"
    );
    match file_type {
        UNIX_REGULAR_TYPE => validate_regular(fields.has_directory_suffix, mode, dos_attributes)?,
        UNIX_DIRECTORY_TYPE => validate_directory(fields, mode, dos_attributes)?,
        _ => ensure!(false, "ZIP entry has an unsupported Unix file type"),
    }
    Ok(ValidatedType {
        mode,
        is_directory: file_type == UNIX_DIRECTORY_TYPE,
    })
}

fn validate_regular(has_directory_suffix: bool, mode: u16, dos_attributes: u16) -> Result<()> {
    ensure!(
        !has_directory_suffix,
        "regular ZIP entry has a directory suffix"
    );
    ensure!(
        matches!(mode, REGULAR_RESOURCE_MODE | EXECUTABLE_MODE),
        "regular ZIP entry has an unsupported mode"
    );
    ensure!(
        dos_attributes == DOS_REGULAR_ATTRIBUTE,
        "regular ZIP entry has conflicting DOS attributes"
    );
    Ok(())
}

fn validate_directory(fields: TypeFields, mode: u16, dos_attributes: u16) -> Result<()> {
    ensure!(
        fields.has_directory_suffix,
        "directory ZIP entry lacks a directory suffix"
    );
    ensure!(mode == DIRECTORY_MODE, "ZIP directory mode mismatch");
    ensure!(
        dos_attributes == DOS_DIRECTORY_ATTRIBUTE,
        "ZIP directory has conflicting DOS attributes"
    );
    ensure!(
        fields.method == STORED_METHOD
            && fields.crc32 == EMPTY_CRC32
            && fields.compressed_size == EMPTY_ENTRY_BYTES
            && fields.uncompressed_size == EMPTY_ENTRY_BYTES,
        "ZIP directory payload must be empty and stored"
    );
    Ok(())
}
