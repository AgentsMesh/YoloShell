use super::{constants::*, files};
use crate::{CapturedExecutable, capture_current, platform};
use std::{fs::File, os::unix::fs::PermissionsExt};

#[test]
fn captured_identity_hashes_this_process_actual_mapped_image_and_can_be_reverified() {
    let captured = CapturedExecutable::capture().unwrap();
    let opened = platform::open_current().unwrap();
    let metadata = opened.file.metadata().unwrap();
    assert_eq!(captured.image().unwrap().size_bytes(), metadata.len());
    assert_eq!(
        captured.image().unwrap().sha256(),
        files::digest(opened.file)
    );
    captured.verify().unwrap();
    assert_eq!(&capture_current().unwrap(), captured.image().unwrap());
}

#[test]
fn byte_identical_copy_is_not_the_running_mapping_and_cannot_supply_its_identity() {
    let mut opened = platform::open_current().unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(DESCRIPTOR_FILE);
    let mut candidate = File::create(&path).unwrap();
    std::io::copy(&mut opened.file, &mut candidate).unwrap();
    candidate
        .set_permissions(std::fs::Permissions::from_mode(PRIVATE_EXECUTABLE_MODE))
        .unwrap();
    drop(candidate);
    opened.file = File::open(&path).unwrap();
    assert!(CapturedExecutable::from_opened(opened).is_err());
}

#[test]
fn disk_candidate_replacement_cannot_be_mistaken_for_current_executable() {
    let mut opened = platform::open_current().unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(DESCRIPTOR_FILE);
    let replacement = root.path().join(REPLACEMENT_FILE);
    let _old = files::image(&path, SOURCE);
    let _replacement = files::image(&replacement, REPLACEMENT);
    std::fs::rename(&replacement, &path).unwrap();
    opened.file = File::open(&path).unwrap();
    assert!(CapturedExecutable::from_opened(opened).is_err());
    CapturedExecutable::capture().unwrap().verify().unwrap();
}
