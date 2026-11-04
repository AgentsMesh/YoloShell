use super::delivery_fixture::{
    COMPLETE_DELIVERY_SIGNATURE_CALLS, DeliveryFixture, ExtractionKind, FakeRunner,
    INITIAL_EXTRACTION_CALLS, result_error,
};
use super::delivery_recovery_support::copy_tree;
use crate::delivery::{DeliveryCheckpoint, DeliveryHooks, deliver_with};
use anyhow::{Result, bail};
use yolo_release_contracts::{
    DELIVERY_COMMITTED_UNCONFIRMED_ERROR, DELIVERY_EXISTING_OUTPUT_ERROR,
    DELIVERY_PREPARED_UNCONFIRMED_ERROR, DELIVERY_RECOVERY_CONFLICT_ERROR,
};

const INJECTED_CONFIRMATION_FAILURE: &str = "injected delivery confirmation failure";
const RECOVERED_SIGNATURE_CALLS: usize = 3;

const POST_COMMIT_CHECKPOINTS: &[DeliveryCheckpoint] = &[
    DeliveryCheckpoint::PublishedDestinationOpen,
    DeliveryCheckpoint::PublishedDestinationIdentityValidation,
    DeliveryCheckpoint::PublishedSignatureValidation,
    DeliveryCheckpoint::PublishedDeliverySync,
    DeliveryCheckpoint::PublishedParentSync,
    DeliveryCheckpoint::PublishedNamespaceValidation,
    DeliveryCheckpoint::PublishedTreeValidation,
    DeliveryCheckpoint::PublishedArchiveValidation,
    DeliveryCheckpoint::PublishedChecksumValidation,
    DeliveryCheckpoint::PublishedReceiptValidation,
    DeliveryCheckpoint::PublishedCandidateValidation,
];

struct NoopHooks;

impl DeliveryHooks for NoopHooks {}

struct FailAt(DeliveryCheckpoint);

impl DeliveryHooks for FailAt {
    fn checkpoint(&self, checkpoint: DeliveryCheckpoint) -> Result<()> {
        if checkpoint == self.0 {
            bail!(INJECTED_CONFIRMATION_FAILURE);
        }
        Ok(())
    }
}

#[test]
fn every_post_commit_failure_is_idempotently_recovered() {
    for checkpoint in POST_COMMIT_CHECKPOINTS {
        let fixture = DeliveryFixture::new();
        let runner = FakeRunner::new(ExtractionKind::Exact, false);
        let error = result_error(deliver_with(
            fixture.request(),
            &runner,
            &FailAt(*checkpoint),
        ));
        assert!(
            error.contains(DELIVERY_COMMITTED_UNCONFIRMED_ERROR),
            "{checkpoint:?}: {error}"
        );
        assert!(fixture.delivery.is_dir(), "{checkpoint:?}");
        fixture.assert_no_stage();

        let recovered = deliver_with(fixture.request(), &runner, &NoopHooks)
            .unwrap_or_else(|error| panic!("{checkpoint:?}: {error:#}"));
        assert_eq!(recovered, fixture.expected, "{checkpoint:?}");
        assert_eq!(runner.extraction_calls(), INITIAL_EXTRACTION_CALLS);
        assert!(runner.signature_calls() >= COMPLETE_DELIVERY_SIGNATURE_CALLS);
        fixture.assert_no_stage();
    }
}

#[test]
fn durable_prepared_stage_resumes_without_reextracting() {
    let fixture = DeliveryFixture::new();
    let runner = FakeRunner::new(ExtractionKind::Exact, false);
    let error = result_error(deliver_with(
        fixture.request(),
        &runner,
        &FailAt(DeliveryCheckpoint::PreparedBeforeRename),
    ));
    assert!(
        error.contains(DELIVERY_PREPARED_UNCONFIRMED_ERROR),
        "{error}"
    );
    assert!(!fixture.delivery.exists());

    assert_eq!(
        deliver_with(fixture.request(), &runner, &NoopHooks).unwrap(),
        fixture.expected
    );
    assert_eq!(runner.extraction_calls(), INITIAL_EXTRACTION_CALLS);
    assert_eq!(runner.signature_calls(), RECOVERED_SIGNATURE_CALLS);
    fixture.assert_no_stage();
}

#[test]
fn completed_delivery_is_an_idempotent_receipt_bound_retry() {
    let fixture = DeliveryFixture::new();
    let runner = FakeRunner::new(ExtractionKind::Exact, false);
    deliver_with(fixture.request(), &runner, &NoopHooks).unwrap();
    assert_eq!(
        deliver_with(fixture.request(), &runner, &NoopHooks).unwrap(),
        fixture.expected
    );
    assert_eq!(runner.extraction_calls(), INITIAL_EXTRACTION_CALLS);
    assert_eq!(runner.signature_calls(), RECOVERED_SIGNATURE_CALLS);
}

#[test]
fn byte_identical_preexisting_output_without_receipt_is_never_adopted() {
    let source = DeliveryFixture::new();
    let source_runner = FakeRunner::new(ExtractionKind::Exact, false);
    deliver_with(source.request(), &source_runner, &NoopHooks).unwrap();

    let target = DeliveryFixture::new();
    copy_tree(&source.delivery, &target.delivery);
    let target_runner = FakeRunner::new(ExtractionKind::Exact, false);
    let error = result_error(deliver_with(target.request(), &target_runner, &NoopHooks));
    assert!(error.contains(DELIVERY_EXISTING_OUTPUT_ERROR), "{error}");
    assert!(!error.contains(DELIVERY_RECOVERY_CONFLICT_ERROR), "{error}");
    assert_eq!(target_runner.extraction_calls(), 0);
    assert_eq!(target_runner.signature_calls(), 0);
}
