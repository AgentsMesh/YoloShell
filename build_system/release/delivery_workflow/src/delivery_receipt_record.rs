use crate::delivery_identity::DeliveryObjectIdentity;
use crate::delivery_target::DeliveryTarget;
use crate::developer_release_binding::DeveloperReleaseBinding;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_release_contracts::*;
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_foundation::digest::bytes_sha256;
use yolo_release_foundation::json::encode;
use yolo_release_verification::verify::VerifiedDeliveryCandidate;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeliveryReceiptRecord {
    schema: u16,
    parent: DeliveryObjectIdentity,
    destination: String,
    stage_name: String,
    stage: DeliveryObjectIdentity,
    artifact_sha256: String,
    artifact_size: u64,
    provenance_sha256: String,
    developer_release: Option<DeveloperReleaseBinding>,
}

impl DeliveryReceiptRecord {
    pub fn capture(
        target: &DeliveryTarget,
        stage_name: &str,
        stage: &DirectoryHandle,
        candidate: &VerifiedDeliveryCandidate,
    ) -> Result<Self> {
        Self::capture_with(target, stage_name, stage, candidate, None)
    }

    pub fn capture_developer_release(
        target: &DeliveryTarget,
        stage_name: &str,
        stage: &DirectoryHandle,
        candidate: &VerifiedDeliveryCandidate,
        binding: DeveloperReleaseBinding,
    ) -> Result<Self> {
        Self::capture_with(target, stage_name, stage, candidate, Some(binding))
    }

    fn capture_with(
        target: &DeliveryTarget,
        stage_name: &str,
        stage: &DirectoryHandle,
        candidate: &VerifiedDeliveryCandidate,
        developer_release: Option<DeveloperReleaseBinding>,
    ) -> Result<Self> {
        Ok(Self {
            schema: DELIVERY_RECEIPT_SCHEMA,
            parent: DeliveryObjectIdentity::capture(&target.parent)?,
            destination: target.destination.clone(),
            stage_name: stage_name.to_owned(),
            stage: DeliveryObjectIdentity::capture(stage)?,
            artifact_sha256: candidate.provenance().artifact.sha256.clone(),
            artifact_size: candidate.provenance().artifact.size_bytes,
            provenance_sha256: provenance_digest(candidate)?,
            developer_release,
        })
    }

    pub fn validate(
        &self,
        target: &DeliveryTarget,
        candidate: &VerifiedDeliveryCandidate,
    ) -> Result<()> {
        ensure!(
            self.schema == DELIVERY_RECEIPT_SCHEMA,
            DELIVERY_RECEIPT_ERROR
        );
        ensure!(self.parent.matches(&target.parent)?, DELIVERY_RECEIPT_ERROR);
        ensure!(
            self.destination == target.destination,
            DELIVERY_RECEIPT_ERROR
        );
        ensure!(
            self.stage_name.starts_with(DELIVERY_TEMPORARY_PREFIX)
                && !self.stage_name.contains('/'),
            DELIVERY_RECEIPT_ERROR
        );
        ensure!(
            self.artifact_sha256 == candidate.provenance().artifact.sha256
                && self.artifact_size == candidate.provenance().artifact.size_bytes
                && self.provenance_sha256 == provenance_digest(candidate)?,
            DELIVERY_RECEIPT_ERROR
        );
        Ok(())
    }

    pub fn validate_stage(&self, stage: &DirectoryHandle) -> Result<()> {
        ensure!(self.stage.matches(stage)?, DELIVERY_RECOVERY_CONFLICT_ERROR);
        Ok(())
    }

    pub fn validate_delivery_kind(&self) -> Result<()> {
        ensure!(self.developer_release.is_none(), DELIVERY_RECEIPT_ERROR);
        Ok(())
    }

    pub fn developer_release(&self) -> Result<&DeveloperReleaseBinding> {
        self.developer_release
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(DELIVERY_RECEIPT_ERROR))
    }

    pub fn stage_name(&self) -> &str {
        &self.stage_name
    }

    pub fn bytes(&self) -> Result<Vec<u8>> {
        encode(self)
    }
}

fn provenance_digest(candidate: &VerifiedDeliveryCandidate) -> Result<String> {
    Ok(bytes_sha256(&encode(candidate.provenance())?))
}
