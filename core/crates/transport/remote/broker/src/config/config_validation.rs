use super::constants::{MAX_HOST_ID_BYTES, SSH_AUTH_SOCK_ENVIRONMENT};
use anyhow::{Result, ensure};
use std::path::PathBuf;

const PROFILE_AUTHORITY_SHA256_ERROR: &str = "invalid profile authority SHA-256";
const CONFIG_REVISION_ERROR: &str = "invalid broker configuration revision";
const SHA256_HEX_BYTES: usize = 64;

pub(super) fn default_use_agent() -> bool {
    true
}

pub(super) fn resolve_agent_socket(
    enabled: bool,
    configured: Option<PathBuf>,
) -> Result<Option<PathBuf>> {
    if !enabled {
        return Ok(None);
    }
    if configured.is_some() {
        return Ok(configured);
    }
    Ok(std::env::var_os(SSH_AUTH_SOCK_ENVIRONMENT).map(PathBuf::from))
}

pub(super) fn validate_host_id(value: &str) -> Result<()> {
    let valid = !value.is_empty()
        && value.len() <= MAX_HOST_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    ensure!(valid, "host_id is empty or unsafe");
    Ok(())
}

pub(crate) fn validate_config_revision(value: &str) -> Result<()> {
    ensure!(is_sha256(value), CONFIG_REVISION_ERROR);
    Ok(())
}

pub(super) fn validate_profile_authority_sha256(value: &str) -> Result<()> {
    ensure!(is_sha256(value), PROFILE_AUTHORITY_SHA256_ERROR);
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == SHA256_HEX_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
