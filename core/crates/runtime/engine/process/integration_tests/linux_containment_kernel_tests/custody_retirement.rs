use super::super::{constants, descriptor};
use super::custody_retirement_fixture::Fixture;
use super::support::SleepingChild;
use std::fs::{File, Permissions};
use std::os::unix::fs::PermissionsExt;

const NO_ACCESS_MODE: u32 = 0;

#[test]
fn retained_custody_proves_empty_after_the_exact_leaf_is_removed() {
    let mut fixture = Fixture::new();
    assert!(!fixture.custody.is_empty().unwrap());
    fixture.retire();
    assert!(!fixture.path.exists());
    assert!(fixture.custody.is_empty().unwrap());
    fixture.custody.request_force_termination().unwrap();
}

#[test]
fn replacement_at_the_old_name_does_not_replace_retained_custody() {
    let mut fixture = Fixture::new();
    let root = File::from(
        fixture
            .custody
            .root_descriptor()
            .try_clone_to_owned()
            .unwrap(),
    );
    let name = fixture.custody.record().leaf_name.clone();
    fixture.retire();
    let replacement = descriptor::create_leaf(&root, &name).unwrap();
    let mut child = SleepingChild::spawn();
    std::fs::write(
        fixture.path.join(constants::CGROUP_PROCESSES_FILE),
        child.identity().pid.to_string(),
    )
    .unwrap();
    let original_is_empty = fixture.custody.is_empty().unwrap();
    fixture.custody.request_force_termination().unwrap();
    let replacement_is_live = child.identity().matches();
    child.terminate();
    descriptor::remove_exact_leaf(&root, &replacement, &name).unwrap();
    assert!(original_is_empty);
    assert!(replacement_is_live);
    assert!(fixture.custody.is_empty().unwrap());
}

#[test]
fn unreadable_membership_is_not_proof_of_removed_custody() {
    let mut fixture = Fixture::new();
    let membership = fixture.path.join(constants::CGROUP_PROCESSES_FILE);
    let original = std::fs::metadata(&membership).unwrap().permissions();
    std::fs::set_permissions(&membership, Permissions::from_mode(NO_ACCESS_MODE)).unwrap();
    let result = fixture.custody.is_empty();
    std::fs::set_permissions(&membership, original).unwrap();
    assert!(result.is_err());
    assert!(!fixture.custody.is_empty().unwrap());
    fixture.retire();
    assert!(fixture.custody.is_empty().unwrap());
}
