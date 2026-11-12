use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_release_acceptance_receipt::constants::SHA256_HEX_CHARACTERS;
use yolo_release_contracts::DELIVERY_RECEIPT_ERROR;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeveloperReleaseBinding {
    archive_sha256: String,
    archive_size: u64,
    notarization_id: String,
    acceptance_receipt_sha256: Option<String>,
}

impl DeveloperReleaseBinding {
    pub fn capture(
        archive_sha256: &str,
        archive_size: u64,
        notarization_id: String,
        acceptance_receipt_sha256: Option<String>,
    ) -> Result<Self> {
        ensure!(!notarization_id.trim().is_empty(), DELIVERY_RECEIPT_ERROR);
        if let Some(digest) = &acceptance_receipt_sha256 {
            validate_digest(digest)?;
        }
        Ok(Self {
            archive_sha256: archive_sha256.to_owned(),
            archive_size,
            notarization_id,
            acceptance_receipt_sha256,
        })
    }

    pub fn validate(&self, archive_sha256: &str, archive_size: u64) -> Result<()> {
        ensure!(
            self.archive_sha256 == archive_sha256
                && self.archive_size == archive_size
                && !self.notarization_id.trim().is_empty(),
            DELIVERY_RECEIPT_ERROR
        );
        Ok(())
    }

    pub fn acceptance_receipt_sha256(&self) -> Option<&str> {
        self.acceptance_receipt_sha256.as_deref()
    }
}

fn validate_digest(value: &str) -> Result<()> {
    ensure!(value.len() == SHA256_HEX_CHARACTERS, DELIVERY_RECEIPT_ERROR);
    ensure!(
        value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        DELIVERY_RECEIPT_ERROR
    );
    Ok(())
}
