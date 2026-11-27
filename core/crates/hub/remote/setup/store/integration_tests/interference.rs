use super::{constants::*, fixture::Fixture};
use std::os::unix::fs::{PermissionsExt, symlink};
use yolo_hub_remote_setup_store::{SetupObservation, SetupOperation, SetupPhase};
use yolo_paths::directory::PrivateDirectory;

#[test]
fn copied_record_cannot_replace_a_retained_record_and_poisoned_owner_cannot_resume() {
    let fixture = Fixture::new();
    let mut operation = fixture.fresh();
    let observation = SetupObservation::observe(&fixture.parent, fixture.id())
        .unwrap()
        .unwrap();
    let moved = fixture.root.path().join(MOVED_NAME);
    std::fs::rename(fixture.file(), &moved).unwrap();
    std::fs::copy(&moved, fixture.file()).unwrap();
    assert!(observation.verify().is_err());
    assert!(operation.verify().is_err());
    assert!(
        operation
            .transition(SetupPhase::Provisioning {
                broker_directory: fixture.broker
            })
            .is_err()
    );
    std::fs::remove_file(fixture.file()).unwrap();
    std::fs::rename(moved, fixture.file()).unwrap();
    assert!(operation.verify().is_err());
}

#[test]
fn replaced_request_or_parent_directory_cannot_adopt_identical_journal_bytes() {
    for replace_parent in [false, true] {
        let fixture = Fixture::new();
        let operation = fixture.fresh();
        let observed = SetupObservation::observe(&fixture.parent, fixture.id())
            .unwrap()
            .unwrap();
        let original = if replace_parent {
            fixture.parent.path().to_owned()
        } else {
            fixture.file().parent().unwrap().to_owned()
        };
        let saved = fixture.root.path().join(MOVED_NAME);
        let bytes = std::fs::read(fixture.file()).unwrap();
        std::fs::rename(&original, &saved).unwrap();
        PrivateDirectory::ensure(&original).unwrap();
        PrivateDirectory::ensure(fixture.file().parent().unwrap()).unwrap();
        std::fs::write(fixture.file(), bytes).unwrap();
        std::fs::set_permissions(
            fixture.file(),
            std::fs::Permissions::from_mode(PRIVATE_MODE),
        )
        .unwrap();
        assert!(observed.verify().is_err());
        assert!(operation.verify().is_err());
        drop(operation);
        let parent = PrivateDirectory::open(fixture.parent.path()).unwrap();
        assert!(SetupObservation::observe(&parent, fixture.id()).is_err());
        assert!(SetupOperation::open(parent, fixture.record.clone()).is_err());
    }
}

#[test]
fn symlinked_entry_or_record_is_neither_followed_nor_repaired() {
    let fixture = Fixture::new();
    let foreign = PrivateDirectory::ensure(&fixture.root.path().join(FOREIGN_NAME)).unwrap();
    let entry = fixture.file().parent().unwrap().to_owned();
    symlink(foreign.path(), &entry).unwrap();
    assert!(SetupObservation::observe(&fixture.parent, fixture.id()).is_err());
    assert!(
        SetupOperation::open(fixture.parent.try_clone().unwrap(), fixture.record.clone()).is_err()
    );
    assert_eq!(
        std::fs::read_dir(foreign.path()).unwrap().count(),
        usize::default()
    );
    let fixture = Fixture::new();
    let operation = fixture.fresh();
    drop(operation);
    let saved = fixture.root.path().join(MOVED_NAME);
    std::fs::rename(fixture.file(), &saved).unwrap();
    symlink(&saved, fixture.file()).unwrap();
    assert!(SetupObservation::observe(&fixture.parent, fixture.id()).is_err());
}
