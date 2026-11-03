use crate::{MAX_AUTH_BYTES, TOKEN_HEX_BYTES};
use anyhow::{Result, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// Only the digest is retained. The caller owns secret file access and issuance.
pub struct AccessToken([u8; 32]);

impl AccessToken {
    pub fn from_hex(secret: &str) -> Result<Self> {
        ensure!(
            secret.len() == TOKEN_HEX_BYTES && secret.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "web token must be 32 random bytes encoded as lowercase hex"
        );
        ensure!(
            !secret.bytes().any(|byte| byte.is_ascii_uppercase()),
            "web token must use lowercase hex"
        );
        Ok(Self(Sha256::digest(secret.as_bytes()).into()))
    }

    pub fn matches(&self, candidate: &str) -> bool {
        let digest: [u8; 32] = Sha256::digest(candidate.as_bytes()).into();
        self.0.ct_eq(&digest).into()
    }

    pub(crate) fn authenticate(&self, bytes: &[u8]) -> Result<()> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Login {
            token: String,
        }
        ensure!(bytes.len() <= MAX_AUTH_BYTES, "web authentication rejected");
        let login: Login = yolo_hub_jsonrpc_protocol::decode(bytes)
            .map_err(|_| anyhow::anyhow!("web authentication rejected"))?;
        let secret = Zeroizing::new(login.token);
        ensure!(self.matches(&secret), "web authentication rejected");
        Ok(())
    }
}

impl std::fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AccessToken(REDACTED)")
    }
}
