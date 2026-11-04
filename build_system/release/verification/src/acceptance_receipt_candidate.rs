use crate::verify::VerifiedDeliveryCandidate;
use anyhow::Result;
use std::path::Path;
use yolo_release_acceptance_receipt::{
    AcceptanceCandidateBinding, AcceptanceExpectation, AcceptanceReceipt,
};
use yolo_release_foundation::digest::bytes_sha256;
use yolo_release_foundation::json::encode;

pub fn candidate_binding_for(
    candidate: &VerifiedDeliveryCandidate,
) -> Result<AcceptanceCandidateBinding> {
    let provenance = encode(candidate.provenance())?;
    AcceptanceCandidateBinding::new(
        candidate.provenance().source_revision.clone(),
        candidate.provenance().artifact.sha256.clone(),
        bytes_sha256(&provenance),
    )
}

pub fn verify_file_for_candidate_at(
    path: &Path,
    candidate: &VerifiedDeliveryCandidate,
    now_unix_seconds: u64,
) -> Result<AcceptanceReceipt> {
    candidate.revalidate()?;
    let receipt = AcceptanceReceipt::open(path)?;
    verify_receipt_for_candidate_at(&receipt, candidate, now_unix_seconds)?;
    Ok(receipt)
}

#[cfg(any(test, feature = "yolo-test-support"))]
pub fn verify_file_for_candidate_at_with_test_key(
    path: &Path,
    candidate: &VerifiedDeliveryCandidate,
    now_unix_seconds: u64,
    public_key: &[u8],
) -> Result<AcceptanceReceipt> {
    candidate.revalidate()?;
    let receipt = yolo_release_acceptance_receipt::open_with_test_key(path, public_key)?;
    verify_receipt_for_candidate_at(&receipt, candidate, now_unix_seconds)?;
    Ok(receipt)
}

pub(crate) fn verify_receipt_for_candidate_at(
    receipt: &AcceptanceReceipt,
    candidate: &VerifiedDeliveryCandidate,
    now_unix_seconds: u64,
) -> Result<()> {
    verify_receipt_for_candidate_at_with_probe(receipt, candidate, now_unix_seconds, || {})
}

pub fn verify_receipt_for_candidate_without_freshness(
    receipt: &AcceptanceReceipt,
    candidate: &VerifiedDeliveryCandidate,
) -> Result<()> {
    let record = receipt.record();
    let expectation = AcceptanceExpectation::new(
        candidate_binding_for(candidate)?,
        record.checklist_schema.clone(),
        record.providers.clone(),
        record.desktop_os.clone(),
        record.remote_os.clone(),
        record.operator_identity.clone(),
        record.completed_at_unix_seconds,
    )?;
    verify_for_candidate(receipt, candidate, &expectation)
}

#[cfg(any(test, feature = "yolo-test-support"))]
pub fn verify_receipt_for_candidate_at_with_test_probe(
    receipt: &AcceptanceReceipt,
    candidate: &VerifiedDeliveryCandidate,
    now_unix_seconds: u64,
    probe: impl FnOnce(),
) -> Result<()> {
    verify_receipt_for_candidate_at_with_probe(receipt, candidate, now_unix_seconds, probe)
}

fn verify_receipt_for_candidate_at_with_probe(
    receipt: &AcceptanceReceipt,
    candidate: &VerifiedDeliveryCandidate,
    now_unix_seconds: u64,
    probe: impl FnOnce(),
) -> Result<()> {
    let record = receipt.record();
    let expectation = AcceptanceExpectation::new(
        candidate_binding_for(candidate)?,
        record.checklist_schema.clone(),
        record.providers.clone(),
        record.desktop_os.clone(),
        record.remote_os.clone(),
        record.operator_identity.clone(),
        now_unix_seconds,
    )?;
    verify_for_candidate(receipt, candidate, &expectation)?;
    probe();
    candidate.revalidate()?;
    receipt.canonical_digest()?;
    Ok(())
}

fn verify_for_candidate(
    receipt: &AcceptanceReceipt,
    candidate: &VerifiedDeliveryCandidate,
    expectation: &AcceptanceExpectation,
) -> Result<()> {
    anyhow::ensure!(
        expectation.candidate == candidate_binding_for(candidate)?,
        yolo_release_acceptance_receipt::constants::RECEIPT_BINDING_ERROR
    );
    receipt.verify(expectation)
}
