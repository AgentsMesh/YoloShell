use super::delivery_fixture::DeliveryFixture;
use crate::delivery_receipt::receipt_name;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use yolo_release_contracts::DELIVERY_RECEIPT_TEMPORARY_PREFIX;
use yolo_release_contracts::DELIVERY_TEMPORARY_PREFIX;
use yolo_release_delivery_foundation::{
    delivery_stage::DeliveryStage, delivery_target::DeliveryTarget,
};
use yolo_release_delivery_workflow::delivery_receipt::StagedDeliveryReceipt;
use yolo_release_delivery_workflow::delivery_receipt_record::DeliveryReceiptRecord;
use yolo_release_verification::verify::VerifiedDeliveryCandidate;

const EXPECTED_STAGE_COUNT: usize = 1;
const EXPECTED_TEMPORARY_RECEIPT_COUNT: usize = 1;

pub fn receipt_path(destination: &Path) -> PathBuf {
    let component = destination.file_name().unwrap().to_str().unwrap();
    destination.parent().unwrap().join(receipt_name(component))
}

pub fn sole_stage(destination: &Path) -> PathBuf {
    let stages = fs::read_dir(destination.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(DELIVERY_TEMPORARY_PREFIX)
        })
        .collect::<Vec<_>>();
    assert_eq!(stages.len(), EXPECTED_STAGE_COUNT);
    stages.into_iter().next().unwrap()
}

pub fn staged_receipt(
    fixture: &DeliveryFixture,
) -> (DeliveryTarget, DeliveryStage, StagedDeliveryReceipt) {
    let target = DeliveryTarget::prepare(&fixture.delivery).unwrap();
    let stage = DeliveryStage::create(&target.parent).unwrap();
    let candidate = VerifiedDeliveryCandidate::open(&fixture.candidate).unwrap();
    let record =
        DeliveryReceiptRecord::capture(&target, stage.name().unwrap(), stage.handle(), &candidate)
            .unwrap();
    let receipt = StagedDeliveryReceipt::create(&target, record).unwrap();
    (target, stage, receipt)
}

pub fn temporary_receipt(fixture: &DeliveryFixture) -> PathBuf {
    let parent: &Path = fixture.delivery.parent().unwrap();
    let files: Vec<_> = fs::read_dir(parent)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(DELIVERY_RECEIPT_TEMPORARY_PREFIX)
        })
        .collect();
    assert_eq!(files.len(), EXPECTED_TEMPORARY_RECEIPT_COUNT);
    files.into_iter().next().unwrap()
}

pub fn copy_tree(source: &Path, destination: &Path) {
    let metadata = fs::symlink_metadata(source).unwrap();
    assert!(!metadata.file_type().is_symlink());
    if metadata.is_dir() {
        fs::create_dir(destination).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            copy_tree(&entry.path(), &destination.join(entry.file_name()));
        }
    } else {
        fs::copy(source, destination).unwrap();
    }
    fs::set_permissions(
        destination,
        fs::Permissions::from_mode(metadata.permissions().mode()),
    )
    .unwrap();
}
