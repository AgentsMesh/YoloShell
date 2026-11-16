mod constants;
use anyhow::{Result, ensure};
pub use constants::{MAX_PROFILE_ID_BYTES, PROFILE_REVISION_HEX_BYTES};
use serde::{Deserialize, Serialize};

/// Public reference only. Neither key material nor a transient broker connection identity.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "ProfileFields", into = "ProfileFields")]
pub struct SshProfileRef(ProfileFields);

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileFields {
    profile_id: String,
    authority_sha256: String,
}

impl SshProfileRef {
    pub fn new(profile_id: impl Into<String>, authority_sha256: impl Into<String>) -> Result<Self> {
        ProfileFields {
            profile_id: profile_id.into(),
            authority_sha256: authority_sha256.into(),
        }
        .try_into()
    }
    pub fn profile_id(&self) -> &str {
        &self.0.profile_id
    }
    pub fn authority_sha256(&self) -> &str {
        &self.0.authority_sha256
    }
}

impl TryFrom<ProfileFields> for SshProfileRef {
    type Error = anyhow::Error;
    fn try_from(fields: ProfileFields) -> Result<Self> {
        use constants::*;
        ensure!(
            !fields.profile_id.is_empty()
                && fields.profile_id.len() <= MAX_PROFILE_ID_BYTES
                && ![CURRENT_DIRECTORY, PARENT_DIRECTORY].contains(&fields.profile_id.as_str())
                && fields
                    .profile_id
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')),
            INVALID_PROFILE
        );
        ensure!(
            fields.authority_sha256.len() == PROFILE_REVISION_HEX_BYTES
                && fields
                    .authority_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')),
            INVALID_PROFILE
        );
        Ok(Self(fields))
    }
}

impl From<SshProfileRef> for ProfileFields {
    fn from(profile: SshProfileRef) -> Self {
        profile.0
    }
}
