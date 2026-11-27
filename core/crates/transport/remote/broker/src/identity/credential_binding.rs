use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub(super) const CREDENTIAL_BINDING_ERROR: &str = "invalid SSH credential binding";
pub(super) const CREDENTIAL_BINDING_MISMATCH: &str = "SSH credential binding mismatch";
const SHA256_HEX_BYTES: usize = 64;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CredentialBindingFile {
    pub(super) nonce: String,
    pub(super) identity_paths_sha256: String,
}

impl CredentialBindingFile {
    pub(super) fn validate(&self) -> Result<()> {
        validate_nonce(&self.nonce)?;
        validate_digest(&self.identity_paths_sha256)
    }
}

pub(super) fn validate_nonce(value: &str) -> Result<()> {
    let parsed = uuid::Uuid::parse_str(value).context(CREDENTIAL_BINDING_ERROR)?;
    ensure!(
        parsed.get_version() == Some(uuid::Version::Random),
        CREDENTIAL_BINDING_ERROR
    );
    Ok(())
}

pub(super) fn validate_digest(value: &str) -> Result<()> {
    ensure!(
        value.len() == SHA256_HEX_BYTES
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')),
        CREDENTIAL_BINDING_ERROR
    );
    Ok(())
}

pub(super) fn ordered_identity_paths_sha256(paths: &[PathBuf]) -> Result<String> {
    let mut material = Vec::new();
    for path in paths {
        let bytes = path_bytes(path);
        let length = u64::try_from(bytes.len()).context(CREDENTIAL_BINDING_ERROR)?;
        material.extend_from_slice(&length.to_be_bytes());
        material.extend_from_slice(bytes);
    }
    Ok(yolo_crypto::sha256_hex(&material))
}

#[cfg(unix)]
fn path_bytes(path: &Path) -> &[u8] {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes()
}
