use super::{constants::*, fixtures::*, parent};
use yolo_hub_enrollment_store::{Admission, ENROLLMENT_DIRECTORY, claim, inspect};
use yolo_paths::directory::PrivateDirectory;

#[test]
fn replaced_entry_directory_cannot_be_adopted_or_completed_even_with_identical_record_bytes() {
    let (root, parent) = parent();
    let prepared = prepared(false);
    let Admission::Fresh(claim) = claim(
        parent.try_clone().unwrap(),
        prepared.peer,
        &prepared.request,
    )
    .unwrap() else {
        panic!()
    };
    let permit = claim.prepare(prepared.clone()).unwrap();
    let observed = inspect(&parent, prepared.peer, &prepared.request)
        .unwrap()
        .unwrap();
    let entry = parent
        .path()
        .join(ENROLLMENT_DIRECTORY)
        .join(prepared.request.fields().request_id.to_string());
    let moved = root.path().join(MOVED_DIRECTORY);
    std::fs::rename(&entry, &moved).unwrap();
    PrivateDirectory::ensure(&entry).unwrap();
    std::fs::copy(moved.join(RECORD_FILE), entry.join(RECORD_FILE)).unwrap();
    assert!(observed.verify().is_err());
    assert!(permit.complete(completed(&prepared)).is_err());
    assert!(inspect(&parent, prepared.peer, &prepared.request).is_err());
}

#[test]
fn same_bytes_record_replacement_fails_the_completion_cas_and_readonly_observation() {
    let (root, parent) = parent();
    let prepared = prepared(false);
    let Admission::Fresh(claim) = claim(
        parent.try_clone().unwrap(),
        prepared.peer,
        &prepared.request,
    )
    .unwrap() else {
        panic!()
    };
    let permit = claim.prepare(prepared.clone()).unwrap();
    let observed = inspect(&parent, prepared.peer, &prepared.request)
        .unwrap()
        .unwrap();
    let file = parent
        .path()
        .join(ENROLLMENT_DIRECTORY)
        .join(prepared.request.fields().request_id.to_string())
        .join(RECORD_FILE);
    let moved = root.path().join(MOVED_DIRECTORY);
    std::fs::rename(&file, &moved).unwrap();
    std::fs::copy(&moved, &file).unwrap();
    assert!(observed.verify().is_err());
    assert!(permit.complete(completed(&prepared)).is_err());
}
