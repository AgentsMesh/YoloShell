use super::*;
use crate::{ChildLaunch, SupervisionRecord};
use std::process::Command;
use yolo_process::ProcessIdentity;
use yolo_remote_broker::{BrokerPaths, BrokerSupervisionStore};

const TRUE_PATH: &str = crate::test_executables::TRUE;
const PREVIOUS_BOOT: &str = "previous-verified-boot";

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    options: ChildLaunch,
    pub(super) store: BrokerSupervisionStore,
    pub(super) family: Family,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let state = root.path().join("state");
        let paths = BrokerPaths::new(&state).unwrap();
        let authority = uuid::Uuid::new_v4();
        let options = crate::launch_fixture::for_paths(&paths, authority);
        let launch_id = authority.to_string();
        let store = BrokerSupervisionStore::open_expected(&state, paths.state_anchor()).unwrap();
        let record = format!("{RECORD_PREFIX}{launch_id}{RECORD_SUFFIX}");
        let family = Family::parse(&record).unwrap();
        Self {
            _root: root,
            options,
            store,
            family,
        }
    }

    pub(super) fn intent(&self, supervisor: ProcessIdentity) -> SupervisionRecord {
        let launcher = ProcessIdentity::capture(std::process::id()).unwrap();
        SupervisionRecord::intent(&self.options, supervisor, launcher).unwrap()
    }

    fn current_intent(&self) -> SupervisionRecord {
        self.intent(ProcessIdentity::capture(std::process::id()).unwrap())
    }

    pub(super) fn record_missing(&self) -> bool {
        self.store
            .read::<serde_json::Value>(&self.family.record)
            .is_err_and(|error| io_kind(&error) == Some(std::io::ErrorKind::NotFound))
    }
}

fn exited_identity() -> ProcessIdentity {
    let mut child = Command::new(TRUE_PATH).spawn().unwrap();
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    child.wait().unwrap();
    identity
}

#[test]
fn request_only_family_is_removed() {
    let fixture = Fixture::new();
    fixture
        .store
        .write(
            &fixture.family.adoption,
            &serde_json::json!({"request": true}),
        )
        .unwrap();
    run(&fixture.store).unwrap();
    assert!(
        fixture
            .store
            .read::<serde_json::Value>(&fixture.family.adoption)
            .is_err()
    );
}

#[test]
fn live_and_corrupt_records_are_retained_fail_closed() {
    let live = Fixture::new();
    live.store
        .write(&live.family.record, &live.current_intent())
        .unwrap();
    run(&live.store).unwrap();
    assert!(!live.record_missing());

    let corrupt = Fixture::new();
    corrupt
        .store
        .write(
            &corrupt.family.record,
            &serde_json::json!({"corrupt": true}),
        )
        .unwrap();
    run(&corrupt.store).unwrap();
    assert!(!corrupt.record_missing());
}

#[test]
fn previous_boot_and_dead_intent_are_removed() {
    let previous = Fixture::new();
    let mut encoded = serde_json::to_value(previous.current_intent()).unwrap();
    encoded[crate::fixture_constants::BOOT_ID_FIELD] = PREVIOUS_BOOT.into();
    previous
        .store
        .write(&previous.family.record, &encoded)
        .unwrap();
    run(&previous.store).unwrap();
    assert!(previous.record_missing());

    let dead = Fixture::new();
    dead.store
        .write(&dead.family.record, &dead.intent(exited_identity()))
        .unwrap();
    run(&dead.store).unwrap();
    assert!(dead.record_missing());
}

#[test]
fn noncanonical_family_name_is_ignored() {
    let fixture = Fixture::new();
    let invalid = format!("{RECORD_PREFIX}{RECORD_SUFFIX}");
    fixture
        .store
        .write(&invalid, &serde_json::json!({"residue": true}))
        .unwrap();
    run(&fixture.store).unwrap();
    assert!(fixture.store.read::<serde_json::Value>(&invalid).is_ok());
}
