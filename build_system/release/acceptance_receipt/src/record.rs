use crate::constants::*;
use crate::record_validation::{
    validate_binding_text, validate_check_result, validate_digest, validate_platform,
    validate_provider, validate_providers, validate_results,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceProviderVersion {
    pub provider: String,
    pub version: String,
}

impl AcceptanceProviderVersion {
    pub fn new(provider: impl Into<String>, version: impl Into<String>) -> Result<Self> {
        let value = Self {
            provider: provider.into(),
            version: version.into(),
        };
        validate_provider(&value)?;
        Ok(value)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptancePlatform {
    pub name: String,
    pub version: String,
    pub architecture: String,
}

impl AcceptancePlatform {
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        architecture: impl Into<String>,
    ) -> Result<Self> {
        let value = Self {
            name: name.into(),
            version: version.into(),
            architecture: architecture.into(),
        };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        validate_platform(self)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceCheckResult {
    pub identifier: String,
    pub status: String,
}

impl AcceptanceCheckResult {
    pub fn passed(identifier: impl Into<String>) -> Result<Self> {
        let value = Self {
            identifier: identifier.into(),
            status: ACCEPTANCE_CHECK_STATUS_PASSED.to_owned(),
        };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        validate_check_result(self)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceCandidateBinding {
    pub source_revision: String,
    pub artifact_sha256: String,
    pub provenance_sha256: String,
}

impl AcceptanceCandidateBinding {
    pub fn new(
        source_revision: impl Into<String>,
        artifact_sha256: impl Into<String>,
        provenance_sha256: impl Into<String>,
    ) -> Result<Self> {
        let value = Self {
            source_revision: source_revision.into(),
            artifact_sha256: artifact_sha256.into(),
            provenance_sha256: provenance_sha256.into(),
        };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        validate_binding_text(&self.source_revision)?;
        validate_digest(&self.artifact_sha256)?;
        validate_digest(&self.provenance_sha256)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceRecord {
    pub schema: u16,
    pub product: String,
    pub source_revision: String,
    pub artifact_sha256: String,
    pub provenance_sha256: String,
    pub checklist_schema: String,
    pub providers: Vec<AcceptanceProviderVersion>,
    pub desktop_os: AcceptancePlatform,
    pub remote_os: AcceptancePlatform,
    pub operator_identity: String,
    pub completed_at_unix_seconds: u64,
    pub expires_at_unix_seconds: u64,
    pub results: Vec<AcceptanceCheckResult>,
}

impl AcceptanceRecord {
    pub(crate) fn validate_shape(&self) -> Result<()> {
        ensure!(
            self.schema == ACCEPTANCE_RECORD_SCHEMA,
            RECEIPT_SCHEMA_ERROR
        );
        ensure!(self.product == ACCEPTANCE_PRODUCT, RECEIPT_PRODUCT_ERROR);
        yolo_release_foundation::validation::validate_revision(&self.source_revision)
            .map_err(|_| anyhow::anyhow!(RECEIPT_BINDING_ERROR))?;
        validate_digest(&self.artifact_sha256)?;
        validate_digest(&self.provenance_sha256)?;
        ensure!(
            self.checklist_schema == ACCEPTANCE_CHECKLIST_SCHEMA,
            RECEIPT_CHECKLIST_ERROR
        );
        validate_providers(&self.providers)?;
        self.desktop_os.validate()?;
        self.remote_os.validate()?;
        validate_binding_text(&self.operator_identity)?;
        validate_results(&self.results)?;
        ensure!(
            self.expires_at_unix_seconds > self.completed_at_unix_seconds,
            RECEIPT_TIME_ERROR
        );
        ensure!(
            self.expires_at_unix_seconds - self.completed_at_unix_seconds
                <= ACCEPTANCE_MAX_VALIDITY_SECONDS,
            RECEIPT_TIME_ERROR
        );
        Ok(())
    }
}
