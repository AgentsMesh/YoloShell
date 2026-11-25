use super::delivery_fixture::{DeliveryFixture, ExtractionKind, FakeRunner, result_error};
use super::delivery_recovery_support::{copy_tree, receipt_path, sole_stage};
use crate::delivery::{DeliveryCheckpoint, DeliveryHooks, deliver_with};
use anyhow::{Result, bail};
use std::fs;
use yolo_release_contracts::{
    DELIVERY_COMMITTED_UNCONFIRMED_ERROR, DELIVERY_PREPARED_UNCONFIRMED_ERROR,
    DELIVERY_RECEIPT_SCHEMA, DELIVERY_RECOVERY_CONFLICT_ERROR,
};
use yolo_release_delivery_foundation::delivery_target::DeliveryTarget;
use yolo_release_delivery_workflow::delivery_receipt::DeliveryReceipt;
use yolo_release_delivery_workflow::delivery_receipt_record::DeliveryReceiptRecord;
use yolo_release_verification::verify::VerifiedDeliveryCandidate;

const ORIGINAL_PARENT: &str = "original-parent";
const RENAMED_PARENT: &str = "renamed-parent";
const RETAINED_OBJECT: &str = "retained-object";
const STOP_MESSAGE: &str = "stop after durable delivery intent";
const RETIRED_RECEIPT_SCHEMA: u16 = 2;
const RETIRED_IDENTITY: &str = r#"{"device":1,"inode":1,"birth_seconds":1,"birth_nanoseconds":0}"#;
const RECOVERY_PHASES: &[(DeliveryCheckpoint, &str)] = &[
    (
        DeliveryCheckpoint::PreparedBeforeRename,
        DELIVERY_PREPARED_UNCONFIRMED_ERROR,
    ),
    (
        DeliveryCheckpoint::PublishedDestinationOpen,
        DELIVERY_COMMITTED_UNCONFIRMED_ERROR,
    ),
];

struct StopAt(DeliveryCheckpoint);
impl DeliveryHooks for StopAt {
    fn checkpoint(&self, checkpoint: DeliveryCheckpoint) -> Result<()> {
        if checkpoint == self.0 {
            bail!(STOP_MESSAGE);
        }
        Ok(())
    }
}

struct Continue;
impl DeliveryHooks for Continue {}

#[test]
fn prepared_and_committed_receipts_reopen_the_same_renamed_parent() {
    for (checkpoint, expected_error) in RECOVERY_PHASES {
        let mut fixture = DeliveryFixture::new();
        let root = tempfile::tempdir().unwrap();
        let parent = root.path().join(ORIGINAL_PARENT);
        fs::create_dir(&parent).unwrap();
        fixture.delivery = parent.join(fixture.delivery.file_name().unwrap());
        let initial = FakeRunner::new(ExtractionKind::Exact, false);
        let error = result_error(deliver_with(
            fixture.request(),
            &initial,
            &StopAt(*checkpoint),
        ));
        assert!(error.contains(expected_error), "{error}");
        let receipt_bytes = fs::read(receipt_path(&fixture.delivery)).unwrap();
        let renamed = root.path().join(RENAMED_PARENT);
        fs::rename(&parent, &renamed).unwrap();
        fixture.delivery = renamed.join(fixture.delivery.file_name().unwrap());

        let target = DeliveryTarget::prepare(&fixture.delivery).unwrap();
        let candidate = VerifiedDeliveryCandidate::open(&fixture.candidate).unwrap();
        let receipt = DeliveryReceipt::open(&target).unwrap().unwrap();
        receipt.validate(&target, &candidate).unwrap();
        drop((receipt, candidate, target));
        let recovery = FakeRunner::new(ExtractionKind::Exact, false);
        assert_eq!(
            deliver_with(fixture.request(), &recovery, &Continue).unwrap(),
            fixture.expected
        );
        assert_eq!(recovery.extraction_calls(), 0);
        assert_eq!(
            fs::read(receipt_path(&fixture.delivery)).unwrap(),
            receipt_bytes
        );
        fixture.assert_no_stage();
    }
}

#[test]
fn byte_identical_stage_or_destination_copy_cannot_replace_receipt_bound_object() {
    for (checkpoint, expected_error) in RECOVERY_PHASES {
        let fixture = DeliveryFixture::new();
        let initial = FakeRunner::new(ExtractionKind::Exact, false);
        let error = result_error(deliver_with(
            fixture.request(),
            &initial,
            &StopAt(*checkpoint),
        ));
        assert!(error.contains(expected_error), "{error}");
        let object = if fixture.delivery.exists() {
            fixture.delivery.clone()
        } else {
            sole_stage(&fixture.delivery)
        };
        let retained = object.with_file_name(RETAINED_OBJECT);
        fs::rename(&object, &retained).unwrap();
        copy_tree(&retained, &object);
        let receipt = fs::read(receipt_path(&fixture.delivery)).unwrap();
        let recovery = FakeRunner::new(ExtractionKind::Exact, false);
        let error = result_error(deliver_with(fixture.request(), &recovery, &Continue));
        assert!(error.contains(expected_error), "{error}");
        assert!(error.contains(DELIVERY_RECOVERY_CONFLICT_ERROR), "{error}");
        assert_eq!(recovery.extraction_calls(), 0);
        assert_eq!(recovery.signature_calls(), 0);
        assert!(object.is_dir());
        assert!(retained.is_dir());
        assert_eq!(fs::read(receipt_path(&fixture.delivery)).unwrap(), receipt);
    }
}

#[test]
fn current_receipt_requires_durable_proofs_and_rejects_retired_schema() {
    let fixture = DeliveryFixture::new();
    deliver_with(
        fixture.request(),
        &FakeRunner::new(ExtractionKind::Exact, false),
        &Continue,
    )
    .unwrap();
    let bytes = fs::read(receipt_path(&fixture.delivery)).unwrap();
    let original: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(original["schema"], DELIVERY_RECEIPT_SCHEMA);
    for field in ["parent", "stage"] {
        assert!(!original[field].as_object().unwrap().contains_key("device"));
        let mut retired = original.clone();
        retired[field] = serde_json::from_str(RETIRED_IDENTITY).unwrap();
        assert!(serde_json::from_value::<DeliveryReceiptRecord>(retired).is_err());
    }
    let mut retired = original;
    retired["schema"] = RETIRED_RECEIPT_SCHEMA.into();
    let record: DeliveryReceiptRecord = serde_json::from_value(retired).unwrap();
    let target = DeliveryTarget::prepare(&fixture.delivery).unwrap();
    let candidate = VerifiedDeliveryCandidate::open(&fixture.candidate).unwrap();
    assert!(record.validate(&target, &candidate).is_err());
    assert_eq!(fs::read(receipt_path(&fixture.delivery)).unwrap(), bytes);
}
