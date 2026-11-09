use super::{constants::*, fixture::Fixture};
use yolo_hub_remote_setup_store::{SetupObservation, SetupOperation, SetupPhase};

#[test]
fn readonly_observation_of_absence_creates_neither_request_directory_nor_lock() {
    let fixture = Fixture::new();
    assert_eq!(
        std::fs::read_dir(fixture.parent.path()).unwrap().count(),
        usize::default()
    );
    assert!(
        SetupObservation::observe(&fixture.parent, fixture.id())
            .unwrap()
            .is_none()
    );
    assert!(
        SetupOperation::inspect(&fixture.parent, fixture.id())
            .unwrap()
            .is_none()
    );
    assert_eq!(
        std::fs::read_dir(fixture.parent.path()).unwrap().count(),
        usize::default()
    );
    assert!(SetupObservation::observe(&fixture.parent, uuid::Uuid::nil()).is_err());
}

#[test]
fn observation_does_not_take_the_mutation_lock_and_cannot_hide_a_phase_change() {
    let fixture = Fixture::new();
    let mut operation = fixture.fresh();
    let before = std::fs::read(fixture.file()).unwrap();
    let observation = SetupObservation::observe(&fixture.parent, fixture.id())
        .unwrap()
        .unwrap();
    assert_eq!(observation.record(), operation.record());
    assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
    observation.verify().unwrap();
    operation
        .transition(SetupPhase::Provisioning {
            broker_directory: fixture.broker,
        })
        .unwrap();
    assert!(observation.verify().is_err());
    let current = SetupObservation::observe(&fixture.parent, fixture.id())
        .unwrap()
        .unwrap();
    current.verify().unwrap();
    assert_eq!(current.record().broker_directory(), Some(fixture.broker));
}

#[test]
fn incomplete_existing_entry_fails_closed_without_repair() {
    let fixture = Fixture::new();
    let entry = fixture.file().parent().unwrap().to_owned();
    yolo_paths::directory::PrivateDirectory::ensure(&entry).unwrap();
    assert!(SetupObservation::observe(&fixture.parent, fixture.id()).is_err());
    assert!(
        SetupOperation::open(fixture.parent.try_clone().unwrap(), fixture.record.clone()).is_err()
    );
    assert!(!entry.join(RECORD_NAME).exists());
}

#[test]
fn stale_parent_cannot_observe_or_create_locks_in_its_replacement() {
    let fixture = Fixture::new();
    let moved = fixture.root.path().join(MOVED_NAME);
    std::fs::rename(fixture.parent.path(), moved).unwrap();
    yolo_paths::directory::PrivateDirectory::ensure(fixture.parent.path()).unwrap();
    assert!(SetupObservation::observe(&fixture.parent, fixture.id()).is_err());
    assert!(
        SetupOperation::open(fixture.parent.try_clone().unwrap(), fixture.record.clone()).is_err()
    );
    assert_eq!(
        std::fs::read_dir(fixture.parent.path()).unwrap().count(),
        usize::default()
    );
}
