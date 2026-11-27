use super::constants::*;
use crate::{CapturedExecutable, capture_current, platform};
use std::os::unix::fs::symlink;

#[test]
fn launch_symlink_is_accepted_only_while_it_names_the_actual_mapped_executable() {
    let directory = tempfile::tempdir().unwrap();
    let alias = directory.path().join(ALIAS_FILE);
    let actual = std::env::current_exe().unwrap().canonicalize().unwrap();
    symlink(&actual, &alias).unwrap();
    let captured =
        CapturedExecutable::from_opened(platform::open_named(alias.clone()).unwrap()).unwrap();
    assert_eq!(captured.image().unwrap(), &capture_current().unwrap());
    captured.verify().unwrap();
    let copy = directory.path().join(REPLACEMENT_FILE);
    std::fs::copy(&actual, &copy).unwrap();
    std::fs::rename(&alias, directory.path().join(SAVED_ALIAS_FILE)).unwrap();
    symlink(&copy, &alias).unwrap();
    assert!(captured.verify().is_err());
    assert!(captured.image().is_err());
    assert!(CapturedExecutable::from_opened(platform::open_named(alias).unwrap()).is_err());
}

#[test]
fn resolving_a_byte_identical_copy_never_substitutes_for_current_process_attestation() {
    let directory = tempfile::tempdir().unwrap();
    let copy = directory.path().join(REPLACEMENT_FILE);
    std::fs::copy(std::env::current_exe().unwrap(), &copy).unwrap();
    let alias = directory.path().join(ALIAS_FILE);
    symlink(&copy, &alias).unwrap();
    let opened = platform::open_named(alias).unwrap();
    assert!(CapturedExecutable::from_opened(opened).is_err());
    capture_current().unwrap();
}
