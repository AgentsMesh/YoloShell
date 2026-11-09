use crate::remote::constants::{
    FSYNC_EXTENSION, FSYNC_EXTENSION_VERSION, POSIX_RENAME_EXTENSION, POSIX_RENAME_VERSION,
    SFTP_PATH_LENGTH_BYTES,
};
use russh_sftp::protocol::{StatusCode, Version};

pub fn server_version(fsync: bool, posix_rename: bool) -> Version {
    let mut version = Version::new();
    if fsync {
        version
            .extensions
            .insert(FSYNC_EXTENSION.into(), FSYNC_EXTENSION_VERSION.into());
    }
    if posix_rename {
        version
            .extensions
            .insert(POSIX_RENAME_EXTENSION.into(), POSIX_RENAME_VERSION.into());
    }
    version
}

pub fn decode_path(data: &[u8]) -> Result<String, StatusCode> {
    let mut remaining = data;
    let path = take_path(&mut remaining)?;
    if !remaining.is_empty() {
        return Err(StatusCode::BadMessage);
    }
    Ok(path)
}

pub fn decode_path_pair(data: &[u8]) -> Result<(String, String), StatusCode> {
    let mut remaining = data;
    let old = take_path(&mut remaining)?;
    let new = take_path(&mut remaining)?;
    if !remaining.is_empty() {
        return Err(StatusCode::BadMessage);
    }
    Ok((old, new))
}

fn take_path(remaining: &mut &[u8]) -> Result<String, StatusCode> {
    if remaining.len() < SFTP_PATH_LENGTH_BYTES {
        return Err(StatusCode::BadMessage);
    }
    let length = u32::from_be_bytes(
        remaining[..SFTP_PATH_LENGTH_BYTES]
            .try_into()
            .map_err(|_| StatusCode::BadMessage)?,
    ) as usize;
    let end = SFTP_PATH_LENGTH_BYTES
        .checked_add(length)
        .filter(|end| *end <= remaining.len())
        .ok_or(StatusCode::BadMessage)?;
    let value = std::str::from_utf8(&remaining[SFTP_PATH_LENGTH_BYTES..end])
        .map_err(|_| StatusCode::BadMessage)?
        .to_owned();
    *remaining = &remaining[end..];
    Ok(value)
}
