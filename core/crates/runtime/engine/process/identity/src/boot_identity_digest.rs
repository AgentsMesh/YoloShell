use super::{BootIdentityRelation, boot_id, boot_identity_relation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const DIGEST_BYTES: usize = 32;
const DIGEST_HEX_LENGTH: usize = DIGEST_BYTES * crate::constants::HEX_DIGITS_PER_BYTE;
const HEX_RADIX: u32 = 16;
const ENCODED_PREFIX: &str = "boot-v1-";
const HASH_DOMAIN: &[u8] = b"yolo-boot-id-sha256-v1\0";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum BootIdentityFormat {
    #[serde(rename = "boot-id-sha256-v1")]
    Current,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootIdentity {
    format: BootIdentityFormat,
    digest: [u8; DIGEST_BYTES],
    known: bool,
}

impl BootIdentity {
    pub fn current() -> Self {
        Self::from_value(&boot_id())
    }

    pub fn from_value(value: &str) -> Self {
        Self {
            format: BootIdentityFormat::Current,
            digest: digest(value),
            known: boot_identity_relation(value, value) == BootIdentityRelation::Same,
        }
    }

    pub fn from_encoded(known: bool, encoded: &str) -> Option<Self> {
        let encoded = encoded.strip_prefix(ENCODED_PREFIX)?;
        if encoded.len() != DIGEST_HEX_LENGTH
            || !encoded
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return None;
        }
        let mut digest = [0_u8; DIGEST_BYTES];
        for (slot, pair) in digest.iter_mut().zip(
            encoded
                .as_bytes()
                .chunks_exact(crate::constants::HEX_DIGITS_PER_BYTE),
        ) {
            *slot = u8::from_str_radix(std::str::from_utf8(pair).ok()?, HEX_RADIX).ok()?;
        }
        Some(Self {
            format: BootIdentityFormat::Current,
            digest,
            known,
        })
    }

    pub fn encoded(self) -> String {
        let digest: String = self
            .digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        format!("{ENCODED_PREFIX}{digest}")
    }

    pub fn known(self) -> bool {
        self.known
    }

    pub fn relation_to_current(self) -> BootIdentityRelation {
        let current = Self::current();
        if !self.known || !current.known {
            BootIdentityRelation::Unknown
        } else if self.digest == current.digest {
            BootIdentityRelation::Same
        } else {
            BootIdentityRelation::Different
        }
    }
}

fn digest(value: &str) -> [u8; DIGEST_BYTES] {
    let mut digest = Sha256::new();
    digest.update(HASH_DOMAIN);
    digest.update(value.as_bytes());
    digest.finalize().into()
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/boot_identity_digest_tests.rs"]
mod tests;
