use super::developer_release_acceptance_support::{GOLDEN_RECEIPT_NOW, write_acceptance_receipt};
use super::developer_release_fixture::DeveloperReleaseFixture;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use yolo_release_acceptance_receipt::{
    AcceptanceRecord, open_with_test_key, signed_with_test_authority, test_public_key,
};
use yolo_release_contracts::{ARCHIVE_NAME, PRIVATE_FILE_MODE, READ_ONLY_FILE_MODE};
use yolo_release_verification::test_support::unseal;
use yolo_release_verification::verify_acceptance::{
    VerifyAcceptanceRequest, verify_acceptance_with_test_key_at,
};

const REPLACED_RECEIPT_NAME: &str = "retained-acceptance.json";
const REPLACED_ARCHIVE_NAME: &str = "retained-candidate.zip";
const DIFFERENT_REVISION: &str = "fedcba9876543210fedcba9876543210fedcba98";
const DIFFERENT_ARTIFACT_DIGEST_CHARACTER: char = '0';
const DIFFERENT_PROVENANCE_DIGEST_CHARACTER: char = '1';

#[derive(Clone, Copy)]
pub enum CandidateBindingDrift {
    SourceRevision,
    ArtifactDigest,
    ProvenanceDigest,
}

pub struct VerifyAcceptanceFixture {
    pub release: DeveloperReleaseFixture,
    pub receipt: PathBuf,
}

impl VerifyAcceptanceFixture {
    pub fn new() -> Self {
        let release = DeveloperReleaseFixture::new();
        let receipt = write_acceptance_receipt(&release);
        Self { release, receipt }
    }

    pub fn request(&self) -> VerifyAcceptanceRequest<'_> {
        VerifyAcceptanceRequest {
            candidate_directory: &self.release.delivery.candidate,
            acceptance_receipt: &self.receipt,
        }
    }

    pub fn verify_at(&self, now: u64) -> anyhow::Result<AcceptanceRecord> {
        verify_acceptance_with_test_key_at(self.request(), now, test_public_key())
    }

    pub fn opened_candidate(&self) -> yolo_release_verification::verify::VerifiedDeliveryCandidate {
        yolo_release_verification::verify::VerifiedDeliveryCandidate::open(
            &self.release.delivery.candidate,
        )
        .unwrap()
    }

    pub fn opened_receipt(&self) -> yolo_release_acceptance_receipt::AcceptanceReceipt {
        open_with_test_key(&self.receipt, test_public_key()).unwrap()
    }

    pub fn drift_binding(&self, drift: CandidateBindingDrift) {
        let mut record = self.opened_receipt().record().clone();
        match drift {
            CandidateBindingDrift::SourceRevision => {
                record.source_revision = DIFFERENT_REVISION.to_owned();
            }
            CandidateBindingDrift::ArtifactDigest => {
                record.artifact_sha256 = different_digest(DIFFERENT_ARTIFACT_DIGEST_CHARACTER);
            }
            CandidateBindingDrift::ProvenanceDigest => {
                record.provenance_sha256 = different_digest(DIFFERENT_PROVENANCE_DIGEST_CHARACTER);
            }
        }
        write_receipt(&self.receipt, signed_with_test_authority(record).unwrap());
    }

    pub fn replace_receipt_path(&self) {
        let bytes = fs::read(&self.receipt).unwrap();
        fs::rename(
            &self.receipt,
            self.receipt.with_file_name(REPLACED_RECEIPT_NAME),
        )
        .unwrap();
        write_receipt(&self.receipt, bytes);
    }

    pub fn replace_candidate_path(&self) {
        unseal(&self.release.delivery.candidate).unwrap();
        fs::rename(
            &self.release.delivery.candidate,
            &self.release.delivery.moved_candidate,
        )
        .unwrap();
        fs::create_dir(&self.release.delivery.candidate).unwrap();
    }

    pub fn replace_artifact_path(&self) {
        unseal(&self.release.delivery.candidate).unwrap();
        let archive = self.release.delivery.candidate.join(ARCHIVE_NAME);
        let bytes = fs::read(&archive).unwrap();
        fs::rename(&archive, archive.with_file_name(REPLACED_ARCHIVE_NAME)).unwrap();
        fs::write(&archive, bytes).unwrap();
        fs::set_permissions(&archive, fs::Permissions::from_mode(READ_ONLY_FILE_MODE)).unwrap();
    }

    pub fn assert_no_output(&self) {
        assert!(!self.release.delivery.delivery.exists());
        self.release.delivery.assert_no_stage();
    }
}

pub const ACCEPTANCE_NOW: u64 = GOLDEN_RECEIPT_NOW;

fn different_digest(character: char) -> String {
    character
        .to_string()
        .repeat(yolo_release_acceptance_receipt::constants::SHA256_HEX_CHARACTERS)
}

fn write_receipt(path: &Path, bytes: Vec<u8>) {
    if path.exists() {
        fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    }
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(READ_ONLY_FILE_MODE)).unwrap();
}
