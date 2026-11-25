use super::*;
use crate::{ChildLaunch, SupervisionRecord};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use yolo_process::ProcessIdentity;
use yolo_remote_broker::{BrokerPaths, BrokerSupervisionStore};

const TRUE_PATH: &str = crate::test_executables::TRUE;
const REQUEST_FAMILIES_PER_ACTIVE_RECORD: usize = 4;
const REQUEST_FAMILY_OVERFLOW: usize = 1;
const REQUEST_FAMILY_COUNT: usize =
    MAX_ACTIVE_RECORDS * REQUEST_FAMILIES_PER_ACTIVE_RECORD + REQUEST_FAMILY_OVERFLOW;
const PRIVATE_FILE_MODE: u32 = 0o600;

fn setup() -> (tempfile::TempDir, BrokerSupervisionStore, ChildLaunch) {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    let paths = BrokerPaths::new(&state).unwrap();
    let options = crate::launch_fixture::for_paths(&paths, uuid::Uuid::new_v4());
    let store = BrokerSupervisionStore::open_expected(&state, paths.state_anchor()).unwrap();
    (root, store, options)
}

fn record_name(launch_id: &str) -> String {
    format!("{RECORD_PREFIX}{launch_id}{RECORD_SUFFIX}")
}

fn live_record(options: &ChildLaunch) -> SupervisionRecord {
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    SupervisionRecord::intent(options, identity, identity).unwrap()
}

#[test]
fn active_record_capacity_fails_closed() {
    let (_root, store, mut options) = setup();
    for _ in 0..MAX_ACTIVE_RECORDS {
        let launch_id = uuid::Uuid::new_v4().to_string();
        options = crate::launch_fixture::with_authority(&options, launch_id.parse().unwrap());
        seed(
            options.state_dir(),
            &record_name(&launch_id),
            &live_record(&options),
        );
    }
    let error = run(&store).unwrap_err();
    assert!(error.to_string().contains(CAPACITY_EXHAUSTED));
}

#[test]
fn request_residue_cannot_starve_a_recoverable_record() {
    let (_root, store, mut options) = setup();
    for _ in 0..REQUEST_FAMILY_COUNT {
        let launch_id = uuid::Uuid::new_v4().to_string();
        let adoption = format!("{RECORD_PREFIX}{launch_id}{ADOPTION_SUFFIX}");
        seed(
            options.state_dir(),
            &adoption,
            &serde_json::json!({"residue": true}),
        );
    }
    let launch_id = uuid::Uuid::new_v4().to_string();
    options = crate::launch_fixture::with_authority(&options, launch_id.parse().unwrap());
    let mut child = Command::new(TRUE_PATH).spawn().unwrap();
    let dead = ProcessIdentity::capture(child.id()).unwrap();
    child.wait().unwrap();
    let launcher = ProcessIdentity::capture(std::process::id()).unwrap();
    let record = SupervisionRecord::intent(&options, dead, launcher).unwrap();
    let name = record_name(&launch_id);
    seed(options.state_dir(), &name, &record);
    run(&store).unwrap();
    assert!(store.read::<serde_json::Value>(&name).is_err());
}

fn seed<T: serde::Serialize>(state: &Path, name: &str, value: &T) {
    let path = state.join(name);
    std::fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
}
