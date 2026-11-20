use crate::output::OutputDirectory;
use crate::output_files::OpenedCandidate;
use crate::validation::{checksum_contents, validate_provenance};
use anyhow::{Result, ensure};
use std::fs::File;
use std::path::Path;
use yolo_release_archive_audit::audit_opened_archive;
use yolo_release_archive_audit::model::ZipInventory;
use yolo_release_contracts::DELIVERY_CANDIDATE_CHANGED_ERROR;
use yolo_release_contracts::{MAX_ARTIFACT_BYTES, PUBLISHED_ARTIFACT_CHANGED_ERROR};
use yolo_release_evidence_model::ReleaseProvenance;
use yolo_release_foundation::digest::opened_file_sha256;
use yolo_release_foundation::json::decode;

pub struct VerifyRequest<'a> {
    pub output_directory: &'a Path,
}

pub struct VerifiedDeliveryCandidate {
    output: OutputDirectory,
    opened: OpenedCandidate,
    provenance: ReleaseProvenance,
    inventory: ZipInventory,
}

pub fn verify(request: VerifyRequest<'_>) -> Result<ReleaseProvenance> {
    let output = OutputDirectory::open(request.output_directory)?;
    verify_opened(&output)
}

pub fn verify_opened(output: &OutputDirectory) -> Result<ReleaseProvenance> {
    let opened = output.open_candidate()?;
    verify_preopened(output, &opened)
}

pub(crate) fn verify_preopened(
    output: &OutputDirectory,
    opened: &OpenedCandidate,
) -> Result<ReleaseProvenance> {
    let artifact = &opened.artifact.file;
    ensure!(
        artifact.metadata()?.len() <= MAX_ARTIFACT_BYTES,
        "published candidate artifact exceeded the bounded limit"
    );
    let artifact_sha256 = opened_file_sha256(&artifact)?;
    let artifact_size = artifact.metadata()?.len();
    let provenance: ReleaseProvenance = decode(&opened.provenance()?)?;
    validate_provenance(&provenance, &artifact_sha256, artifact_size)?;
    ensure!(
        opened.checksum()? == checksum_contents(&artifact_sha256).as_bytes(),
        "checksum file mismatch"
    );
    ensure!(
        opened_file_sha256(&artifact)? == artifact_sha256
            && artifact.metadata()?.len() == artifact_size,
        PUBLISHED_ARTIFACT_CHANGED_ERROR
    );
    output.verify_candidate(&opened)?;
    Ok(provenance)
}

impl VerifiedDeliveryCandidate {
    pub fn open(path: &Path) -> Result<Self> {
        let output = OutputDirectory::open(path)?;
        let opened = output.open_candidate()?;
        let provenance = verify_preopened(&output, &opened)?;
        let inventory = audit_opened_archive(&opened.artifact.file)?;
        let candidate = Self {
            output,
            opened,
            provenance,
            inventory,
        };
        candidate.revalidate()?;
        Ok(candidate)
    }

    pub fn copy_archive(&self, destination: &mut File) -> Result<()> {
        self.revalidate()?;
        yolo_release_candidate_artifact::copy_opened(
            &self.opened.artifact.file,
            destination,
            self.provenance.artifact.size_bytes,
        )?;
        self.revalidate()
    }

    pub fn revalidate(&self) -> Result<()> {
        ensure!(
            verify_preopened(&self.output, &self.opened)? == self.provenance,
            DELIVERY_CANDIDATE_CHANGED_ERROR
        );
        ensure!(
            audit_opened_archive(&self.opened.artifact.file)? == self.inventory,
            DELIVERY_CANDIDATE_CHANGED_ERROR
        );
        Ok(())
    }

    pub fn provenance(&self) -> &ReleaseProvenance {
        &self.provenance
    }

    pub fn inventory(&self) -> &ZipInventory {
        &self.inventory
    }
}
