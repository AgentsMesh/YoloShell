use super::{constants::*, fixtures::*};
use std::path::Path;
use uuid::Uuid;
use yolo_distribution_publication_protocol::*;

#[test]
fn otherwise_valid_receipts_cannot_cross_request_root_or_selected_revision() {
    let request = request();
    let receipt =
        PublicationReceipt::new(&request, Path::new(ROOT), request.selected().unwrap()).unwrap();
    assert!(receipt.validate(&request, Path::new(OTHER_ROOT)).is_err());
    for other in [
        PublicationRequest::new(
            ROOT,
            Uuid::new_v4(),
            request.stage(),
            request.descriptor().clone(),
            None,
        )
        .unwrap(),
        PublicationRequest::new(
            ROOT,
            request.operation_id(),
            Uuid::new_v4(),
            request.descriptor().clone(),
            None,
        )
        .unwrap(),
        PublicationRequest::new(
            ROOT,
            request.operation_id(),
            request.stage(),
            descriptor(OTHER_VERSION),
            None,
        )
        .unwrap(),
    ] {
        assert!(receipt.validate(&other, Path::new(ROOT)).is_err());
    }
    let mut fields = receipt.fields().clone();
    let mut selected = fields.selection.fields().clone();
    selected.revision += 1;
    fields.selection = Selection::try_from(selected).unwrap();
    let changed = PublicationReceipt::try_from(fields).unwrap();
    assert!(changed.validate(&request, Path::new(ROOT)).is_err());
    let mut fields = receipt.fields().clone();
    fields.release_directory = Path::new(OTHER_ROOT).into();
    assert!(
        PublicationReceipt::try_from(fields)
            .unwrap()
            .validate(&request, Path::new(ROOT))
            .is_err()
    );
    let mut fields = receipt.fields().clone();
    fields.operation_id = Uuid::new_v4();
    assert!(PublicationReceipt::try_from(fields).is_err());
    let mut fields = receipt.fields().clone();
    fields.stage = Uuid::nil();
    assert!(PublicationReceipt::try_from(fields).is_err());
    let mut fields = receipt.fields().clone();
    fields.release_directory = Path::new(INVALID_ROOT).into();
    assert!(PublicationReceipt::try_from(fields).is_err());
}

#[test]
fn persisted_request_root_cannot_be_retargeted_by_a_different_cli_argument() {
    let request = request();
    let encoded = encode_request(&request).unwrap();
    let restored = decode_request(&encoded).unwrap();
    assert_eq!(restored.install_root(), Path::new(ROOT));
    assert!(restored.validate_root(Path::new(OTHER_ROOT)).is_err());
    assert!(
        PublicationReceipt::new(
            &restored,
            Path::new(OTHER_ROOT),
            restored.selected().unwrap()
        )
        .is_err()
    );
    let mut fields = restored.fields().clone();
    fields.install_root = INVALID_ROOT.into();
    assert!(PublicationRequest::try_from(fields).is_err());
}

#[test]
fn inspections_require_the_matching_bundle_and_exact_derived_path() {
    let request = request();
    let selected = request.selected().unwrap();
    assert!(
        InspectionReceipt::new(
            Path::new(ROOT),
            Some((selected.clone(), descriptor(OTHER_VERSION)))
        )
        .is_err()
    );
    let valid = InspectionReceipt::new(
        Path::new(ROOT),
        Some((selected, request.descriptor().clone())),
    )
    .unwrap();
    assert!(valid.validate(Path::new(OTHER_ROOT)).is_err());
    assert_eq!(valid.fields().install_root, Path::new(ROOT));
    let mut fields = valid.fields().clone();
    fields.schema_version += 1;
    assert!(InspectionReceipt::try_from(fields).is_err());
    let mut fields = valid.fields().clone();
    let mut entry = fields.selected.take().unwrap().fields().clone();
    entry.release_directory = OTHER_ROOT.into();
    fields.selected = Some(SelectedRelease::try_from(entry).unwrap());
    assert!(InspectionReceipt::try_from(fields).is_err());
    let mut fields = valid.fields().clone();
    fields.install_root = INVALID_ROOT.into();
    assert!(InspectionReceipt::try_from(fields).is_err());
}
