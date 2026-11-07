use super::super::constants::{
    CONFIG_GENERATIONS_DIRECTORY, MANAGEMENT_SOCKET_FILE, RUNTIME_SOCKET_FILE,
};
use super::super::{BrokerPaths, BrokerStateAnchor};
use std::path::Path;

const STATE_DIRECTORY: &str = crate::remote_broker::constants::TEST_STATE_DIRECTORY;
const DISPLACED_DIRECTORY: &str = "state-displaced";
const COORDINATION_DIRECTORY: &str = "coordination";
const INVALID_ANCHORS: [&str; 7] = ["", "1", "1:", ":1", "a:1", "1:a", "1:2:3"];

#[test]
fn anchor_round_trip_and_invalid_forms_are_explicit() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join(STATE_DIRECTORY)).unwrap();
    let encoded = paths.state_anchor().to_string();

    assert_eq!(
        encoded.parse::<BrokerStateAnchor>().unwrap(),
        paths.state_anchor()
    );
    for invalid in INVALID_ANCHORS {
        assert!(invalid.parse::<BrokerStateAnchor>().is_err(), "{invalid}");
    }
}

#[test]
fn expected_anchor_opens_only_the_selected_state_directory() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join(STATE_DIRECTORY);
    let paths = BrokerPaths::new(&state).unwrap();
    let anchor = paths.state_anchor();
    let reopened = BrokerPaths::new_expected(&state, anchor).unwrap();

    assert_eq!(reopened.state_dir(), state);
    assert_eq!(reopened.state_anchor(), anchor);
    assert_eq!(
        reopened.runtime_socket().file_name().unwrap(),
        RUNTIME_SOCKET_FILE
    );
    assert_eq!(
        reopened.management_socket().file_name().unwrap(),
        MANAGEMENT_SOCKET_FILE
    );
    assert_eq!(
        reopened.generations().file_name().unwrap(),
        CONFIG_GENERATIONS_DIRECTORY
    );
    assert_eq!(reopened.instance_lock(), paths.instance_lock());
    assert_eq!(
        reopened.launch_publication_lock(),
        paths.launch_publication_lock()
    );
    assert_eq!(reopened.reconcile_lock(), paths.reconcile_lock());
    reopened.verify_state().unwrap();
    reopened.state_directory().unwrap().verify_path().unwrap();

    let displaced = root.path().join(DISPLACED_DIRECTORY);
    std::fs::rename(&state, displaced).unwrap();
    BrokerPaths::new(&state).unwrap();
    assert!(reopened.verify_state().is_err());
    assert!(BrokerPaths::new_expected(&state, anchor).is_err());
}

#[test]
fn expected_anchor_rejects_non_absolute_state_and_custom_coordination_is_stable() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join(STATE_DIRECTORY);
    let paths = BrokerPaths::new(&state).unwrap();
    assert!(BrokerPaths::new_expected(Path::new(STATE_DIRECTORY), paths.state_anchor()).is_err());

    let coordination = root.path().join(COORDINATION_DIRECTORY);
    let coordinated = BrokerPaths::new_with_coordination(&state, &coordination).unwrap();
    assert_eq!(coordinated.instance_lock().parent().unwrap(), coordination);
    assert_eq!(
        coordinated.launch_publication_lock().parent().unwrap(),
        coordination
    );
    assert_eq!(coordinated.reconcile_lock().parent().unwrap(), coordination);
    assert_eq!(coordinated.state_anchor(), paths.state_anchor());
}
