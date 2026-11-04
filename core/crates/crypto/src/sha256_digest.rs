use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const DIGEST_SERIALIZATION_CONTEXT: &str = "serialize SHA-256 digest source";
const SHA256_DIGEST_BYTES: usize = 32;

#[path = "sha256_digest_compact.rs"]
pub mod compact;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sha256Digest([u8; SHA256_DIGEST_BYTES]);

impl Sha256Digest {
    pub fn capture<T: Serialize>(value: &T) -> Result<Self> {
        let encoded = serde_json::to_vec(value).context(DIGEST_SERIALIZATION_CONTEXT)?;
        Ok(Self(Sha256::digest(encoded).into()))
    }

    #[doc(hidden)]
    pub const fn repeated(value: u8) -> Self {
        Self([value; SHA256_DIGEST_BYTES])
    }
}
