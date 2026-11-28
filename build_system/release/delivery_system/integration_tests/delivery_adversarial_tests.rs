use super::delivery_fixture::{
    DeliveryFixture, ExtractionKind, FakeRunner, MALFORMED_ARCHIVE_BYTES, result_error,
};
use crate::delivery::{DeliveryCheckpoint, DeliveryHooks, deliver_with};
use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use yolo_release_contracts::{DELIVERY_ROOT_ENTRIES_ERROR, DELIVERY_TREE_MISMATCH_ERROR};
use yolo_release_verification::test_support::unseal;

#[test]
fn candidate_path_replacement_after_verification_fails_closed() {
    let fixture = DeliveryFixture::new();
    let hook = ReplaceCandidate {
        candidate: fixture.candidate.clone(),
        moved: fixture.moved_candidate.clone(),
        invoked: AtomicBool::new(false),
    };
    let runner = FakeRunner::new(ExtractionKind::Exact, false);
    let error = result_error(deliver_with(fixture.request(), &runner, &hook));
    assert!(
        error.contains("sealed directory changed") || error.contains("path identity changed"),
        "{error}"
    );
    assert!(hook.invoked.load(Ordering::SeqCst));
    assert!(!fixture.delivery.exists());
    fixture.assert_no_stage();
}

#[test]
fn coherent_candidate_with_malformed_zip_is_reaudited_before_copy_or_extract() {
    let fixture = DeliveryFixture::from_archive(MALFORMED_ARCHIVE_BYTES.to_vec());
    let runner = FakeRunner::new(ExtractionKind::Exact, false);
    let error = result_error(deliver_with(fixture.request(), &runner, &NoopHooks));
    assert!(error.contains("ZIP"), "{error}");
    assert_eq!(runner.extraction_calls(), 0);
    assert!(!fixture.delivery.exists());
    fixture.assert_no_stage();
}

#[test]
fn extra_extracted_root_entry_is_rejected_before_signature_audit() {
    let fixture = DeliveryFixture::new();
    let runner = FakeRunner::new(ExtractionKind::ExtraRoot, false);
    let error = result_error(deliver_with(fixture.request(), &runner, &NoopHooks));
    assert!(error.contains(DELIVERY_ROOT_ENTRIES_ERROR), "{error}");
    assert_eq!(runner.signature_calls(), 0);
    assert!(!fixture.delivery.exists());
    fixture.assert_no_stage();
}

#[test]
fn extracted_permission_drift_is_rejected() {
    let fixture = DeliveryFixture::new();
    let runner = FakeRunner::new(ExtractionKind::PermissionDrift, false);
    let error = result_error(deliver_with(fixture.request(), &runner, &NoopHooks));
    assert!(error.contains(DELIVERY_TREE_MISMATCH_ERROR), "{error}");
    assert_eq!(runner.signature_calls(), 0);
    assert!(!fixture.delivery.exists());
    fixture.assert_no_stage();
}

#[test]
fn concurrent_destination_creation_wins_without_replacement() {
    let fixture = DeliveryFixture::new();
    let hook = CreateDestination {
        destination: fixture.delivery.clone(),
    };
    let runner = FakeRunner::new(ExtractionKind::Exact, false);
    let error = result_error(deliver_with(fixture.request(), &runner, &hook));
    assert!(error.contains("initially be absent"), "{error}");
    assert!(fixture.delivery.is_dir());
    assert_eq!(fs::read_dir(&fixture.delivery).unwrap().count(), 0);
    fixture.assert_no_stage();
}

struct NoopHooks;

impl DeliveryHooks for NoopHooks {}

struct ReplaceCandidate {
    candidate: PathBuf,
    moved: PathBuf,
    invoked: AtomicBool,
}

impl DeliveryHooks for ReplaceCandidate {
    fn checkpoint(&self, checkpoint: DeliveryCheckpoint) -> Result<()> {
        if checkpoint == DeliveryCheckpoint::CandidateVerified {
            unseal(&self.candidate)?;
            fs::rename(&self.candidate, &self.moved)?;
            fs::create_dir(&self.candidate)?;
            self.invoked.store(true, Ordering::SeqCst);
        }
        Ok(())
    }
}

struct CreateDestination {
    destination: PathBuf,
}

impl DeliveryHooks for CreateDestination {
    fn checkpoint(&self, checkpoint: DeliveryCheckpoint) -> Result<()> {
        if checkpoint == DeliveryCheckpoint::BeforePublish {
            fs::create_dir(&self.destination)?;
        }
        Ok(())
    }
}
