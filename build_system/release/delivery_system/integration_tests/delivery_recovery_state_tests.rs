use super::delivery_fixture::{
    DeliveryFixture, ExtractionKind, FakeRunner, INITIAL_EXTRACTION_CALLS, result_error,
};
use super::delivery_recovery_support::{receipt_path, sole_stage};
use crate::delivery::{DeliveryCheckpoint, DeliveryHooks, deliver_with};
use anyhow::{Result, bail};
use std::fs;
use std::path::PathBuf;
use yolo_release_contracts::{
    DELIVERY_COMMITTED_UNCONFIRMED_ERROR, DELIVERY_PREPARED_UNCONFIRMED_ERROR,
    DELIVERY_RECOVERY_CONFLICT_ERROR,
};

const INJECTED_PREPARED_FAILURE: &str = "injected prepared delivery failure";
const RETAINED_STAGE_NAME: &str = "retained-stage-evidence";
const RETAINED_DELIVERY_NAME: &str = "retained-delivery-evidence";

struct NoopHooks;

impl DeliveryHooks for NoopHooks {}

struct FailPrepared;

impl DeliveryHooks for FailPrepared {
    fn checkpoint(&self, checkpoint: DeliveryCheckpoint) -> Result<()> {
        if checkpoint == DeliveryCheckpoint::PreparedBeforeRename {
            bail!(INJECTED_PREPARED_FAILURE);
        }
        Ok(())
    }
}

struct CreateDestinationAfterIntent {
    destination: PathBuf,
}

impl DeliveryHooks for CreateDestinationAfterIntent {
    fn checkpoint(&self, checkpoint: DeliveryCheckpoint) -> Result<()> {
        if checkpoint == DeliveryCheckpoint::PreparedBeforeRename {
            fs::create_dir(&self.destination)?;
        }
        Ok(())
    }
}

#[test]
fn replacement_stage_identity_is_rejected_and_preserved() {
    let fixture = DeliveryFixture::new();
    let runner = prepare(&fixture);
    let stage = sole_stage(&fixture.delivery);
    let retained = stage.parent().unwrap().join(RETAINED_STAGE_NAME);
    fs::rename(&stage, &retained).unwrap();
    fs::create_dir(&stage).unwrap();

    let error = result_error(deliver_with(fixture.request(), &runner, &NoopHooks));
    assert!(
        error.contains(DELIVERY_PREPARED_UNCONFIRMED_ERROR),
        "{error}"
    );
    assert!(error.contains(DELIVERY_RECOVERY_CONFLICT_ERROR), "{error}");
    assert!(stage.is_dir());
    assert!(retained.is_dir());
    assert_eq!(runner.extraction_calls(), INITIAL_EXTRACTION_CALLS);
}

#[test]
fn receipt_with_both_stage_and_destination_fails_closed() {
    let fixture = DeliveryFixture::new();
    let runner = prepare(&fixture);
    let stage = sole_stage(&fixture.delivery);
    fs::create_dir(&fixture.delivery).unwrap();

    let error = result_error(deliver_with(fixture.request(), &runner, &NoopHooks));
    assert!(error.contains(DELIVERY_RECOVERY_CONFLICT_ERROR), "{error}");
    assert!(stage.is_dir());
    assert!(fixture.delivery.is_dir());
    assert!(receipt_path(&fixture.delivery).is_file());
    assert_eq!(runner.extraction_calls(), INITIAL_EXTRACTION_CALLS);
}

#[test]
fn receipt_with_recorded_stage_and_destination_both_absent_fails_closed() {
    let fixture = DeliveryFixture::new();
    let runner = prepare(&fixture);
    let stage = sole_stage(&fixture.delivery);
    let retained = stage.parent().unwrap().join(RETAINED_STAGE_NAME);
    fs::rename(&stage, &retained).unwrap();

    let error = result_error(deliver_with(fixture.request(), &runner, &NoopHooks));
    assert!(error.contains(DELIVERY_RECOVERY_CONFLICT_ERROR), "{error}");
    assert!(!stage.exists());
    assert!(!fixture.delivery.exists());
    assert!(retained.is_dir());
    assert!(receipt_path(&fixture.delivery).is_file());
    assert_eq!(runner.extraction_calls(), INITIAL_EXTRACTION_CALLS);
}

#[test]
fn destination_created_after_durable_intent_is_never_replaced() {
    let fixture = DeliveryFixture::new();
    let runner = FakeRunner::new(ExtractionKind::Exact, false);
    let error = result_error(deliver_with(
        fixture.request(),
        &runner,
        &CreateDestinationAfterIntent {
            destination: fixture.delivery.clone(),
        },
    ));
    assert!(
        error.contains(DELIVERY_PREPARED_UNCONFIRMED_ERROR),
        "{error}"
    );
    let stage = sole_stage(&fixture.delivery);
    assert!(stage.is_dir());
    assert!(fixture.delivery.is_dir());
    assert_eq!(fs::read_dir(&fixture.delivery).unwrap().count(), 0);
    assert!(receipt_path(&fixture.delivery).is_file());

    let retry = result_error(deliver_with(fixture.request(), &runner, &NoopHooks));
    assert!(retry.contains(DELIVERY_RECOVERY_CONFLICT_ERROR), "{retry}");
    assert!(stage.is_dir());
    assert!(fixture.delivery.is_dir());
}

#[test]
fn replacement_of_receipt_bound_destination_is_never_adopted() {
    let fixture = DeliveryFixture::new();
    let runner = FakeRunner::new(ExtractionKind::Exact, false);
    deliver_with(fixture.request(), &runner, &NoopHooks).unwrap();
    let retained = fixture
        .delivery
        .parent()
        .unwrap()
        .join(RETAINED_DELIVERY_NAME);
    fs::rename(&fixture.delivery, &retained).unwrap();
    fs::create_dir(&fixture.delivery).unwrap();

    let error = result_error(deliver_with(fixture.request(), &runner, &NoopHooks));
    assert!(
        error.contains(DELIVERY_COMMITTED_UNCONFIRMED_ERROR),
        "{error}"
    );
    assert!(error.contains(DELIVERY_RECOVERY_CONFLICT_ERROR), "{error}");
    assert!(fixture.delivery.is_dir());
    assert!(retained.is_dir());
}

fn prepare(fixture: &DeliveryFixture) -> FakeRunner {
    let runner = FakeRunner::new(ExtractionKind::Exact, false);
    let error = result_error(deliver_with(fixture.request(), &runner, &FailPrepared));
    assert!(
        error.contains(DELIVERY_PREPARED_UNCONFIRMED_ERROR),
        "{error}"
    );
    assert!(receipt_path(&fixture.delivery).is_file());
    runner
}
