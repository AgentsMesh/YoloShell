use super::delivery_fixture::DeliveryFixture;
use super::delivery_recovery_support::{staged_receipt as staged, temporary_receipt};
use std::fs;
use yolo_release_contracts::DELIVERY_RECEIPT_ERROR;
use yolo_release_delivery_foundation::{
    delivery_stage::DeliveryStage, delivery_target::DeliveryTarget,
};
use yolo_release_delivery_workflow::delivery_receipt::receipt_name;
use yolo_release_delivery_workflow::delivery_receipt_record::DeliveryReceiptRecord;
use yolo_release_verification::verify::VerifiedDeliveryCandidate;

const REPLACEMENT_BYTES: &[u8] = b"replacement receipt must survive";
const MOVED_RECEIPT_NAME: &str = "retained-original-receipt";
const INVALID_STAGE_PREFIX: &str = "unowned-stage";
const INVALID_STAGE_COMPONENT: &str = "/outside";

#[test]
fn dropping_an_unpublished_receipt_removes_only_its_own_temporary_file() {
    let fixture = DeliveryFixture::new();
    let (_target, stage, receipt) = staged(&fixture);
    let temporary = temporary_receipt(&fixture);
    let stage_path = stage.handle().display_path().to_owned();
    drop(receipt);
    assert!(!temporary.exists());
    assert!(stage_path.is_dir());
    assert!(!fixture.delivery.exists());
    drop(stage);
    assert!(!stage_path.exists());
}

#[test]
fn dropping_a_replaced_receipt_preserves_the_replacement_and_retained_original() {
    let fixture = DeliveryFixture::new();
    let (_target, _stage, receipt) = staged(&fixture);
    let temporary = temporary_receipt(&fixture);
    let original_bytes = fs::read(&temporary).unwrap();
    let retained = temporary.with_file_name(MOVED_RECEIPT_NAME);
    fs::rename(&temporary, &retained).unwrap();
    fs::write(&temporary, REPLACEMENT_BYTES).unwrap();
    drop(receipt);
    assert_eq!(fs::read(&temporary).unwrap(), REPLACEMENT_BYTES);
    assert_eq!(fs::read(&retained).unwrap(), original_bytes);
}

#[test]
fn publication_collision_preserves_the_existing_receipt_and_cleans_owned_stage() {
    let fixture = DeliveryFixture::new();
    let (target, mut stage, receipt) = staged(&fixture);
    let temporary = temporary_receipt(&fixture);
    let final_path = target
        .parent
        .display_path()
        .join(receipt_name(&target.destination));
    let stage_path = stage.handle().display_path().to_owned();
    fs::write(&final_path, REPLACEMENT_BYTES).unwrap();
    assert!(receipt.publish(&mut stage).is_err());
    assert_eq!(fs::read(&final_path).unwrap(), REPLACEMENT_BYTES);
    assert!(!temporary.exists());
    drop(stage);
    assert!(!stage_path.exists());
}

#[test]
fn missing_temporary_receipt_cannot_publish_or_retain_the_delivery_stage() {
    let fixture = DeliveryFixture::new();
    let (target, mut stage, receipt) = staged(&fixture);
    let temporary = temporary_receipt(&fixture);
    let retained = temporary.with_file_name(MOVED_RECEIPT_NAME);
    let stage_path = stage.handle().display_path().to_owned();
    fs::rename(&temporary, &retained).unwrap();
    assert_eq!(
        receipt.publish(&mut stage).err().unwrap().to_string(),
        DELIVERY_RECEIPT_ERROR
    );
    assert!(
        !target
            .parent
            .display_path()
            .join(receipt_name(&target.destination))
            .exists()
    );
    assert!(retained.is_file());
    drop(stage);
    assert!(!stage_path.exists());
}

#[test]
fn receipt_record_rejects_schema_and_stage_path_boundary_changes() {
    let fixture = DeliveryFixture::new();
    let target = DeliveryTarget::prepare(&fixture.delivery).unwrap();
    let stage = DeliveryStage::create(&target.parent).unwrap();
    let candidate = VerifiedDeliveryCandidate::open(&fixture.candidate).unwrap();
    let record =
        DeliveryReceiptRecord::capture(&target, stage.name().unwrap(), stage.handle(), &candidate)
            .unwrap();
    let source = serde_json::to_value(&record).unwrap();
    let changes = [
        (
            "schema",
            serde_json::json!(yolo_release_contracts::DELIVERY_RECEIPT_SCHEMA + 1),
        ),
        ("stage_name", serde_json::json!(INVALID_STAGE_PREFIX)),
        (
            "stage_name",
            serde_json::json!(format!(
                "{}{INVALID_STAGE_COMPONENT}",
                stage.name().unwrap()
            )),
        ),
        (
            "artifact_size",
            serde_json::json!(candidate.provenance().artifact.size_bytes + 1),
        ),
        (
            "provenance_sha256",
            serde_json::json!(candidate.provenance().artifact.sha256),
        ),
    ];
    for (field, value) in changes {
        let mut changed = source.clone();
        changed[field] = value;
        let changed: DeliveryReceiptRecord = serde_json::from_value(changed).unwrap();
        assert_eq!(
            changed
                .validate(&target, &candidate)
                .unwrap_err()
                .to_string(),
            DELIVERY_RECEIPT_ERROR
        );
    }
}
