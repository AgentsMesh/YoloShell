use crate::delivery_receipt::DeliveryReceipt;
use crate::developer_release::DeveloperReleaseRequest;
use crate::developer_release_constants::PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR;
use anyhow::{Context, Result};
use yolo_release_acceptance_receipt::AcceptanceReceipt;
use yolo_release_verification::verify::VerifiedDeliveryCandidate;

pub trait AcceptanceVerifier {
    fn authorize(
        &self,
        request: &DeveloperReleaseRequest<'_>,
        candidate: &VerifiedDeliveryCandidate,
        now_unix_seconds: u64,
    ) -> Result<AcceptanceReceipt>;

    fn authorize_recovery(
        &self,
        request: &DeveloperReleaseRequest<'_>,
        candidate: &VerifiedDeliveryCandidate,
        expected_digest: &str,
    ) -> Result<AcceptanceReceipt>;
}

pub struct PinnedAcceptanceVerifier;

impl AcceptanceVerifier for PinnedAcceptanceVerifier {
    fn authorize(
        &self,
        request: &DeveloperReleaseRequest<'_>,
        candidate: &VerifiedDeliveryCandidate,
        now_unix_seconds: u64,
    ) -> Result<AcceptanceReceipt> {
        crate::developer_release_acceptance::authorize(request, candidate, now_unix_seconds)
    }

    fn authorize_recovery(
        &self,
        request: &DeveloperReleaseRequest<'_>,
        candidate: &VerifiedDeliveryCandidate,
        expected_digest: &str,
    ) -> Result<AcceptanceReceipt> {
        crate::developer_release_acceptance::authorize_recovery(request, candidate, expected_digest)
    }
}

#[cfg(test)]
pub struct TestAcceptanceVerifier<'a> {
    public_key: &'a [u8],
}

#[cfg(test)]
impl<'a> TestAcceptanceVerifier<'a> {
    pub fn new(public_key: &'a [u8]) -> Self {
        Self { public_key }
    }
}

#[cfg(test)]
impl AcceptanceVerifier for TestAcceptanceVerifier<'_> {
    fn authorize(
        &self,
        request: &DeveloperReleaseRequest<'_>,
        candidate: &VerifiedDeliveryCandidate,
        now_unix_seconds: u64,
    ) -> Result<AcceptanceReceipt> {
        crate::developer_release_acceptance::authorize_with_test_key(
            request,
            candidate,
            now_unix_seconds,
            self.public_key,
        )
    }

    fn authorize_recovery(
        &self,
        request: &DeveloperReleaseRequest<'_>,
        candidate: &VerifiedDeliveryCandidate,
        expected_digest: &str,
    ) -> Result<AcceptanceReceipt> {
        crate::developer_release_acceptance::authorize_recovery_with_test_key(
            request,
            candidate,
            expected_digest,
            self.public_key,
        )
    }
}

pub fn load_for_delivery(
    verifier: &dyn AcceptanceVerifier,
    request: &DeveloperReleaseRequest<'_>,
    candidate: &VerifiedDeliveryCandidate,
    existing_receipt: Option<&DeliveryReceipt>,
    now_unix_seconds: Option<u64>,
) -> Result<Option<AcceptanceReceipt>> {
    #[cfg(test)]
    if now_unix_seconds.is_none() {
        return Ok(None);
    }
    let now_unix_seconds = now_unix_seconds.context(PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR)?;
    let acceptance = match existing_receipt {
        Some(receipt) => {
            let digest = receipt
                .record()
                .developer_release()?
                .acceptance_receipt_sha256()
                .context(PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR)?;
            verifier.authorize_recovery(request, candidate, digest)
        }
        None => verifier.authorize(request, candidate, now_unix_seconds),
    }?;
    Ok(Some(acceptance))
}
