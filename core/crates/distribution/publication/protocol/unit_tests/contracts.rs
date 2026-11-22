use super::{constants::*, fixtures::*};
use std::path::Path;
use uuid::Uuid;
use yolo_distribution_publication_protocol::*;

#[test]
fn publication_and_inspection_bind_exact_selection_descriptor_and_immutable_paths() {
    let request = request();
    let selected = request.selected().unwrap();
    assert_eq!(selected.revision(), FIRST_REVISION);
    assert_eq!(selected.operation_id(), request.operation_id());
    assert_eq!(selected.build_id(), request.descriptor().build_id());
    let receipt = PublicationReceipt::new(&request, Path::new(ROOT), selected.clone()).unwrap();
    receipt.validate(&request, Path::new(ROOT)).unwrap();
    assert_eq!(receipt.selection(), &selected);
    assert_eq!(receipt.fields().stage, request.stage());
    assert_eq!(
        receipt.release_directory(),
        request.descriptor().release_directory(Path::new(ROOT))
    );
    let inspection = InspectionReceipt::new(
        Path::new(ROOT),
        Some((selected.clone(), request.descriptor().clone())),
    )
    .unwrap();
    inspection.validate(Path::new(ROOT)).unwrap();
    let published = inspection.selected().unwrap();
    assert_eq!(published.selection(), &selected);
    assert_eq!(published.descriptor(), request.descriptor());
    assert_eq!(published.release_directory(), receipt.release_directory());
    assert_eq!(published.fields().selection, selected);
    let replay = PublicationRequest::new(
        ROOT,
        request.operation_id(),
        request.stage(),
        request.descriptor().clone(),
        Some(selected),
    )
    .unwrap();
    assert_eq!(replay.selected().unwrap(), request.selected().unwrap());
    PublicationReceipt::new(&replay, Path::new(ROOT), replay.selected().unwrap()).unwrap();
    assert!(
        InspectionReceipt::new(Path::new(ROOT), None)
            .unwrap()
            .selected()
            .is_none()
    );
}

#[test]
fn requests_reject_nil_identity_revision_exhaustion_and_operation_reuse_for_another_build() {
    for (operation, stage) in [(Uuid::nil(), Uuid::new_v4()), (Uuid::new_v4(), Uuid::nil())] {
        assert!(
            PublicationRequest::new(ROOT, operation, stage, descriptor(VERSION), None).is_err()
        );
    }
    let request = request();
    let mut exhausted = request.selected().unwrap().fields().clone();
    exhausted.revision = u64::MAX;
    let exhausted = Selection::try_from(exhausted).unwrap();
    assert!(
        PublicationRequest::new(
            ROOT,
            Uuid::new_v4(),
            Uuid::new_v4(),
            descriptor(VERSION),
            Some(exhausted)
        )
        .is_err()
    );
    assert!(
        PublicationRequest::new(
            ROOT,
            request.operation_id(),
            request.stage(),
            descriptor(OTHER_VERSION),
            Some(request.selected().unwrap())
        )
        .is_err()
    );
    let upgraded = PublicationRequest::new(
        ROOT,
        Uuid::new_v4(),
        Uuid::new_v4(),
        descriptor(OTHER_VERSION),
        Some(request.selected().unwrap()),
    )
    .unwrap();
    assert_eq!(
        upgraded.selected().unwrap().revision(),
        FIRST_REVISION + REVISION_STEP
    );
}

#[test]
fn remote_paths_are_scoped_bounded_and_lexically_normalized_without_filesystem_access() {
    for root in [
        "/",
        "relative",
        INVALID_ROOT,
        "/srv//yolo",
        "/srv/./yolo",
        "/srv/yolo/",
        "/srv/yolo\0",
        "/srv/yolo\n",
    ] {
        assert!(validate_install_root(Path::new(root)).is_err());
    }
    assert!(
        validate_install_root(Path::new(&format!(
            "/{ROOT}{}",
            "x".repeat(MAX_INSTALL_PATH_BYTES)
        )))
        .is_err()
    );
    for root in [
        ROOT,
        "/srv/yolo with space",
        "/srv/yolo 'quote' $literal `literal`",
    ] {
        validate_install_root(Path::new(root)).unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        assert!(validate_install_root(Path::new(std::ffi::OsStr::from_bytes(&[u8::MAX]))).is_err());
    }
}
