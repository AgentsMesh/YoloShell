use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use zeroize::Zeroizing;

#[derive(Clone, Eq, PartialEq)]
pub struct PrivateCredentials(Zeroizing<Vec<u8>>);
impl PrivateCredentials {
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        let bytes = Zeroizing::new(bytes);
        ensure!(
            !bytes.is_empty() && bytes.len() <= MAX_PRIVATE_CREDENTIAL_BYTES,
            INVALID_CREDENTIALS
        );
        Ok(Self(bytes))
    }
    pub fn into_bytes(self) -> Zeroizing<Vec<u8>> {
        self.0
    }
}
impl std::fmt::Debug for PrivateCredentials {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(PRIVATE_INPUT)
    }
}
impl Serialize for PrivateCredentials {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde_bytes::Bytes::new(&self.0).serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for PrivateCredentials {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = serde_bytes::ByteBuf::deserialize(deserializer)?.into_vec();
        Self::new(bytes).map_err(D::Error::custom)
    }
}
