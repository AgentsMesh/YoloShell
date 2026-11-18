use super::*;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const LEASE_SCHEMA_VERSION: u16 = 1;
const LEASE_CONTENT_ERROR: &str = "socket publication stage lease is not canonical";
const PLACEHOLDER_DIGEST_CHARACTER: char = '0';

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LeaseRecord {
    pub(super) schema_version: u16,
    pub(super) effective_user_id: u32,
    pub(super) target_digest: String,
}

pub(super) fn decode(bytes: &[u8]) -> Result<LeaseRecord> {
    let record: LeaseRecord = serde_json::from_slice(bytes).context(LEASE_CONTENT_ERROR)?;
    anyhow::ensure!(
        record.schema_version == LEASE_SCHEMA_VERSION,
        LEASE_CONTENT_ERROR
    );
    anyhow::ensure!(
        valid_hex(&record.target_digest, FULL_DIGEST_HEX_LENGTH),
        LEASE_CONTENT_ERROR
    );
    anyhow::ensure!(serde_json::to_vec(&record)? == bytes, LEASE_CONTENT_ERROR);
    Ok(record)
}

pub(super) fn encode(target: &Target) -> Result<Vec<u8>> {
    serde_json::to_vec(&from_target(target)).map_err(Into::into)
}

pub(super) fn matches_target(bytes: &[u8], target: &Target) -> Result<bool> {
    Ok(decode(bytes)? == from_target(target))
}

pub(super) fn target_from_key(parent: &Path, key: &str) -> Result<Target> {
    anyhow::ensure!(valid_hex(key, KEY_HEX_LENGTH), LEASE_CONTENT_ERROR);
    let suffix = PLACEHOLDER_DIGEST_CHARACTER
        .to_string()
        .repeat(FULL_DIGEST_HEX_LENGTH - KEY_HEX_LENGTH);
    Target::from_existing_digest(parent, unsafe { libc::geteuid() }, format!("{key}{suffix}"))
}

pub(super) fn is_unproven(error: &anyhow::Error) -> bool {
    error
        .chain()
        .filter_map(|source| source.downcast_ref())
        .all(|io: &std::io::Error| {
            matches!(
                io.raw_os_error(),
                Some(libc::ELOOP)
                    | Some(libc::EISDIR)
                    | Some(libc::ENOTDIR)
                    | Some(libc::EACCES)
                    | Some(libc::EPERM)
            )
        })
}

fn from_target(target: &Target) -> LeaseRecord {
    LeaseRecord {
        schema_version: LEASE_SCHEMA_VERSION,
        effective_user_id: target.effective_user_id,
        target_digest: target.full_digest.clone(),
    }
}
