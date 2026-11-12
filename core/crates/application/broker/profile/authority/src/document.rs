use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

const AUTHORITY_VERSION: u16 = 1;
const SHA256_HEX_BYTES: usize = 64;
const HOST_ID_FIELD: &str = "hostID";

#[derive(Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum AuthorityPhase {
    Blocked,
    Committed,
    Deleted,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AuthorityDocument {
    version: u16,
    #[serde(rename = "hostID")]
    host_id: String,
    phase: AuthorityPhase,
    #[serde(default, rename = "mutationID")]
    mutation_id: Option<Uuid>,
    #[serde(default)]
    profile: Option<Value>,
    #[serde(default, rename = "profileSHA256")]
    profile_sha256: Option<String>,
    #[serde(default, rename = "previousProfile")]
    previous_profile: Option<Value>,
    #[serde(default, rename = "previousProfileSHA256")]
    previous_profile_sha256: Option<String>,
}

impl AuthorityDocument {
    pub(super) fn decode(bytes: &[u8]) -> Result<Self> {
        let value: Value = serde_json::from_slice(bytes)?;
        super::shape::validate(&value)?;
        let document: Self = serde_json::from_value(value)?;
        document.validate()?;
        Ok(document)
    }

    pub(super) fn require_committed(&self, expected_sha256: &str) -> Result<&str> {
        ensure!(
            self.phase == AuthorityPhase::Committed,
            "remote host authority is not committed"
        );
        ensure!(
            self.profile_sha256.as_deref() == Some(expected_sha256),
            "remote host profile authority digest mismatch"
        );
        Ok(&self.host_id)
    }

    pub(super) fn require_blocked(&self, host_id: &str, mutation_id: Uuid) -> Result<()> {
        ensure!(
            self.phase == AuthorityPhase::Blocked,
            "remote host authority is not blocked"
        );
        ensure!(
            self.host_id == host_id,
            "remote host authority host mismatch"
        );
        ensure!(
            self.mutation_id == Some(mutation_id),
            "remote host authority mutation mismatch"
        );
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            self.version == AUTHORITY_VERSION,
            "unsupported remote host authority version"
        );
        ensure!(
            !self.host_id.is_empty(),
            "remote host authority host is empty"
        );
        match self.phase {
            AuthorityPhase::Blocked => {
                ensure!(
                    self.mutation_id.is_some(),
                    "blocked authority has no mutation"
                );
                ensure!(
                    self.profile.is_none() && self.profile_sha256.is_none(),
                    "blocked authority contains a committed profile"
                );
                validate_optional_profile(
                    &self.host_id,
                    self.previous_profile.as_ref(),
                    self.previous_profile_sha256.as_deref(),
                )?;
            }
            AuthorityPhase::Committed => {
                ensure!(
                    self.mutation_id.is_none(),
                    "committed authority contains a mutation"
                );
                ensure!(
                    self.previous_profile.is_none() && self.previous_profile_sha256.is_none(),
                    "committed authority contains a previous profile"
                );
                validate_optional_profile(
                    &self.host_id,
                    self.profile.as_ref(),
                    self.profile_sha256.as_deref(),
                )?;
                ensure!(self.profile.is_some(), "committed authority has no profile");
            }
            AuthorityPhase::Deleted => ensure!(
                self.mutation_id.is_none()
                    && self.profile.is_none()
                    && self.profile_sha256.is_none()
                    && self.previous_profile.is_none()
                    && self.previous_profile_sha256.is_none(),
                "deleted authority contains retained state"
            ),
        }
        Ok(())
    }
}

pub(super) fn validate_digest(value: &str) -> Result<()> {
    ensure!(
        value.len() == SHA256_HEX_BYTES
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')),
        "remote host profile authority digest is invalid"
    );
    Ok(())
}

fn validate_optional_profile(
    host_id: &str,
    profile: Option<&Value>,
    digest: Option<&str>,
) -> Result<()> {
    ensure!(
        profile.is_some() == digest.is_some(),
        "authority profile and digest presence differ"
    );
    if let Some(digest) = digest {
        validate_digest(digest)?;
    }
    if let Some(profile) = profile {
        let object = profile
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("authority profile is not an object"))?;
        ensure!(
            object.get(HOST_ID_FIELD).and_then(Value::as_str) == Some(host_id),
            "authority profile host mismatch"
        );
        let observed = crate::digest::profile_sha256(profile)?;
        ensure!(
            digest == Some(observed.as_str()),
            "authority profile digest mismatch"
        );
    }
    Ok(())
}
