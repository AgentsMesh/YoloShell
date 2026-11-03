use crate::acceptance_receipt_candidate::verify_receipt_for_candidate_at;
use crate::verify::VerifiedDeliveryCandidate;
use anyhow::Result;
use std::path::Path;
use yolo_release_acceptance_receipt::{AcceptanceReceipt, AcceptanceRecord, current_unix_seconds};

pub struct VerifyAcceptanceRequest<'a> {
    pub candidate_directory: &'a Path,
    pub acceptance_receipt: &'a Path,
}

pub fn verify_acceptance(request: VerifyAcceptanceRequest<'_>) -> Result<AcceptanceRecord> {
    verify_acceptance_at(request, current_unix_seconds()?)
}

fn verify_acceptance_at(
    request: VerifyAcceptanceRequest<'_>,
    now_unix_seconds: u64,
) -> Result<AcceptanceRecord> {
    let candidate = VerifiedDeliveryCandidate::open(request.candidate_directory)?;
    let receipt = AcceptanceReceipt::open(request.acceptance_receipt)?;
    verify_receipt_for_candidate_at(&receipt, &candidate, now_unix_seconds)?;
    Ok(receipt.record().clone())
}

#[cfg(any(test, feature = "yolo-test-support"))]
pub fn verify_acceptance_with_test_key_at(
    request: VerifyAcceptanceRequest<'_>,
    now_unix_seconds: u64,
    public_key: &[u8],
) -> Result<AcceptanceRecord> {
    let candidate = VerifiedDeliveryCandidate::open(request.candidate_directory)?;
    let receipt = yolo_release_acceptance_receipt::open_with_test_key(
        request.acceptance_receipt,
        public_key,
    )?;
    verify_receipt_for_candidate_at(&receipt, &candidate, now_unix_seconds)?;
    Ok(receipt.record().clone())
}
