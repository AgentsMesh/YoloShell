use super::{SHA256_DIGEST_BYTES, Sha256Digest};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Deserializer, Serializer};

const COMPACT_DIGEST_ERROR: &str = "compact SHA-256 digest is not canonical";

pub fn serialize<S>(value: &Sha256Digest, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&URL_SAFE_NO_PAD.encode(value.0))
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Sha256Digest, D::Error>
where
    D: Deserializer<'de>,
{
    let encoded = String::deserialize(deserializer)?;
    let decoded = URL_SAFE_NO_PAD
        .decode(&encoded)
        .map_err(|_| serde::de::Error::custom(COMPACT_DIGEST_ERROR))?;
    let bytes: [u8; SHA256_DIGEST_BYTES] = decoded
        .try_into()
        .map_err(|_| serde::de::Error::custom(COMPACT_DIGEST_ERROR))?;
    if URL_SAFE_NO_PAD.encode(bytes) != encoded {
        return Err(serde::de::Error::custom(COMPACT_DIGEST_ERROR));
    }
    Ok(Sha256Digest(bytes))
}
