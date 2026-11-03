use super::*;
use anyhow::Result;
use std::ffi::CStr;
use std::os::unix::ffi::OsStrExt;

const SOCKET_PATH_LENGTH_ERROR: &str = "generated Unix socket path is too long";

pub(super) fn validate(path: &Path) -> Result<()> {
    anyhow::ensure!(
        path.as_os_str().as_bytes().len() < UNIX_SOCKET_PATH_MAX_BYTES,
        "{SOCKET_PATH_LENGTH_ERROR}: {}",
        path.display()
    );
    Ok(())
}

pub(crate) fn is_endpoint_name(name: &CStr) -> bool {
    if name.to_bytes() == HOST_SOCKET_FILE_NAME.as_bytes()
        || name.to_bytes() == HUB_SOCKET_FILE_NAME.as_bytes()
        || name.to_bytes() == CONNECTOR_SOCKET_FILE_NAME.as_bytes()
        || name.to_bytes() == PUBLIC_CLIENT_SOCKET_FILE_NAME.as_bytes()
    {
        return true;
    }
    let Ok(value) = name.to_str() else {
        return false;
    };
    let Some(value) = value.strip_suffix(SOCKET_FILE_EXTENSION) else {
        return false;
    };
    if let Some(value) = value
        .strip_prefix(CLAUDE_HOOK_SOCKET_PREFIX)
        .or_else(|| value.strip_prefix(CODEX_HOOK_SOCKET_PREFIX))
        .or_else(|| value.strip_prefix(QWEN_HOOK_SOCKET_PREFIX))
        .or_else(|| value.strip_prefix(PROVIDER_HOOK_SOCKET_PREFIX))
        .or_else(|| value.strip_prefix(CUSTODY_GUARDIAN_SOCKET_PREFIX))
    {
        return Uuid::parse_str(value).is_ok_and(|id| id.simple().to_string() == value);
    }
    Uuid::parse_str(value).is_ok_and(|id| id.hyphenated().to_string() == value)
}
