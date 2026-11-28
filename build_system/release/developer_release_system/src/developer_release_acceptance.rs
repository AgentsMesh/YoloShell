use crate::developer_release::DeveloperReleaseRequest;
use crate::developer_release_command::validate_profile;
use crate::developer_release_constants::PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR;
use anyhow::{Context, Result, ensure};
use std::path::Path;
use yolo_release_acceptance_receipt::AcceptanceReceipt;
use yolo_release_contracts::CANDIDATE_UNACCEPTED_STATUS;
#[cfg(test)]
use yolo_release_verification::acceptance_receipt_candidate::verify_file_for_candidate_at_with_test_key;
use yolo_release_verification::acceptance_receipt_candidate::{
    verify_file_for_candidate_at, verify_receipt_for_candidate_without_freshness,
};
use yolo_release_verification::verify::VerifiedDeliveryCandidate;

pub fn authorize(
    request: &DeveloperReleaseRequest<'_>,
    candidate: &VerifiedDeliveryCandidate,
    now_unix_seconds: u64,
) -> Result<AcceptanceReceipt> {
    authorize_with_verifier(
        request,
        candidate,
        now_unix_seconds,
        |path, candidate, now| verify_file_for_candidate_at(path, candidate, now),
    )
}

#[cfg(test)]
pub fn authorize_with_test_key(
    request: &DeveloperReleaseRequest<'_>,
    candidate: &VerifiedDeliveryCandidate,
    now_unix_seconds: u64,
    public_key: &[u8],
) -> Result<AcceptanceReceipt> {
    authorize_with_verifier(
        request,
        candidate,
        now_unix_seconds,
        |path, candidate, now| {
            verify_file_for_candidate_at_with_test_key(path, candidate, now, public_key)
        },
    )
}

fn authorize_with_verifier(
    request: &DeveloperReleaseRequest<'_>,
    candidate: &VerifiedDeliveryCandidate,
    now_unix_seconds: u64,
    verify: impl FnOnce(&Path, &VerifiedDeliveryCandidate, u64) -> Result<AcceptanceReceipt>,
) -> Result<AcceptanceReceipt> {
    validate_profile(request.notary_profile)?;
    ensure!(
        candidate.provenance().release_status == CANDIDATE_UNACCEPTED_STATUS,
        PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR
    );
    let receipt = request
        .acceptance_receipt
        .context(PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR)?;
    verify(receipt, candidate, now_unix_seconds).context(PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR)
}

pub fn authorize_recovery(
    request: &DeveloperReleaseRequest<'_>,
    candidate: &VerifiedDeliveryCandidate,
    expected_digest: &str,
) -> Result<AcceptanceReceipt> {
    authorize_recovery_with_opener(request, candidate, expected_digest, AcceptanceReceipt::open)
}

#[cfg(test)]
pub fn authorize_recovery_with_test_key(
    request: &DeveloperReleaseRequest<'_>,
    candidate: &VerifiedDeliveryCandidate,
    expected_digest: &str,
    public_key: &[u8],
) -> Result<AcceptanceReceipt> {
    authorize_recovery_with_opener(request, candidate, expected_digest, |path| {
        yolo_release_acceptance_receipt::open_with_test_key(path, public_key)
    })
}

fn authorize_recovery_with_opener(
    request: &DeveloperReleaseRequest<'_>,
    candidate: &VerifiedDeliveryCandidate,
    expected_digest: &str,
    open: impl FnOnce(&Path) -> Result<AcceptanceReceipt>,
) -> Result<AcceptanceReceipt> {
    candidate.revalidate()?;
    let receipt_path = request
        .acceptance_receipt
        .context(PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR)?;
    let receipt = open(receipt_path).context(PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR)?;
    ensure!(
        receipt.canonical_digest()? == expected_digest,
        PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR
    );
    verify_receipt_for_candidate_without_freshness(&receipt, candidate)
        .context(PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR)?;
    candidate.revalidate()?;
    receipt
        .canonical_digest()
        .context(PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR)?;
    Ok(receipt)
}

pub fn revalidate(
    receipt: Option<&AcceptanceReceipt>,
    candidate: &VerifiedDeliveryCandidate,
) -> Result<()> {
    if let Some(receipt) = receipt {
        receipt
            .canonical_digest()
            .context(PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR)?;
    }
    candidate.revalidate()
}
