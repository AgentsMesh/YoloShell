use crate::delivery_files::ProtectedFile;
use anyhow::{Result, ensure};
use std::sync::Arc;
use yolo_release_archive_audit::audit_opened_archive;
use yolo_release_contracts::ARCHIVE_NAME;
use yolo_release_contracts::{DELIVERY_ARCHIVE_DIGEST_ERROR, DELIVERY_ARCHIVE_INVENTORY_ERROR};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_foundation::digest::opened_file_sha256;
use yolo_release_verification::verify::VerifiedDeliveryCandidate;

pub struct DeliveryArchive {
    file: ProtectedFile,
    digest: String,
}

impl DeliveryArchive {
    pub fn create(root: &DirectoryHandle, candidate: &VerifiedDeliveryCandidate) -> Result<Self> {
        let file = ProtectedFile::create(root, ARCHIVE_NAME, |destination| {
            candidate.copy_archive(destination)
        })?;
        Self::validate(file, candidate)
    }

    pub fn open(root: &DirectoryHandle, candidate: &VerifiedDeliveryCandidate) -> Result<Self> {
        Self::validate(ProtectedFile::open(root, ARCHIVE_NAME)?, candidate)
    }

    fn validate(file: ProtectedFile, candidate: &VerifiedDeliveryCandidate) -> Result<Self> {
        let digest = opened_file_sha256(file.file())?;
        ensure!(
            digest == candidate.provenance().artifact.sha256,
            DELIVERY_ARCHIVE_DIGEST_ERROR
        );
        ensure!(
            audit_opened_archive(file.file())? == *candidate.inventory(),
            DELIVERY_ARCHIVE_INVENTORY_ERROR
        );
        Ok(Self { file, digest })
    }

    pub fn verify_at(
        &self,
        root: &DirectoryHandle,
        candidate: &VerifiedDeliveryCandidate,
    ) -> Result<()> {
        let reopened = self.file.verify_at(root, ARCHIVE_NAME)?;
        ensure!(
            opened_file_sha256(&reopened)? == self.digest,
            DELIVERY_ARCHIVE_DIGEST_ERROR
        );
        ensure!(
            audit_opened_archive(&reopened)? == *candidate.inventory(),
            DELIVERY_ARCHIVE_INVENTORY_ERROR
        );
        Ok(())
    }

    pub fn retained_file(&self) -> Result<Arc<std::fs::File>> {
        Ok(Arc::new(self.file.file().try_clone()?))
    }
}
