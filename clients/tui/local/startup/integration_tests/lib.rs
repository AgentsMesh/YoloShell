#[path = "../test_support/bootstrap_contract.rs"]
mod bootstrap_contract;
mod bootstrap_process;
#[path = "../test_support/bootstrap_process.rs"]
mod bootstrap_process_support;
#[path = "../test_support/constants.rs"]
mod fixture_constants;
#[path = "../test_support/installation.rs"]
mod installation;
use crate::{Helpers, receipt};
use fixture_constants::*;
use installation::Installation;
use std::{fs, os::unix::fs::PermissionsExt};
use yolo_hub_local_startup::{BootstrapDocument, ConnectionDocument};
use yolo_local_profile::LocalProfile;

#[test]
fn executable_alias_resolves_to_one_program_and_requires_safe_executable_permissions() {
    let installation = Installation::new();
    let helpers = Helpers::discover(&installation.alias()).unwrap();
    assert_eq!(
        helpers.executable,
        installation.executable.canonicalize().unwrap()
    );
    fs::remove_file(&helpers.executable).unwrap();
    assert!(Helpers::discover(&installation.executable).is_err());
    assert!(Helpers::discover(installation.root.path()).is_err());
    fs::write(&helpers.executable, b"fixture").unwrap();
    fs::set_permissions(&helpers.executable, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(Helpers::discover(&installation.executable).is_err());
    fs::set_permissions(&helpers.executable, fs::Permissions::from_mode(0o777)).unwrap();
    assert!(Helpers::discover(&installation.executable).is_err());
}

#[test]
fn receipt_requires_the_exact_existing_profile_identity_and_schema() {
    let directory = tempfile::tempdir().unwrap();
    let profile = LocalProfile::from_application_support(directory.path()).unwrap();
    fs::create_dir_all(&profile.host_home).unwrap();
    let state = yolo_hub_state::HubState::open(&profile.state, &profile.host_home).unwrap();
    let connection = ConnectionDocument::local(state.peer_identity().hub(), state.socket_path());
    let document = BootstrapDocument::new(&profile.state, &profile.host_home, connection).unwrap();
    let bytes = yolo_capnp_codec::encode(&document).unwrap();
    assert_eq!(
        receipt::validate(&profile, &bytes).unwrap(),
        document.connection
    );
    for field in [
        SCHEMA_VERSION_FIELD,
        STATE_PATH_FIELD,
        HOST_HOME_FIELD,
        CONNECTION_FIELD,
    ] {
        let mut changed = serde_json::to_value(&document).unwrap();
        changed[field] = serde_json::Value::Null;
        assert!(receipt::validate(&profile, &yolo_capnp_codec::encode(&changed).unwrap()).is_err());
    }
    for (field, value) in [
        (
            WORKSPACE_PATH_FIELD,
            serde_json::json!("/unexpected/workspace"),
        ),
        (
            WORKING_DIRECTORIES_FIELD,
            serde_json::json!({"unexpected": "/remote"}),
        ),
    ] {
        let mut injected = serde_json::to_value(&document).unwrap();
        injected[CONNECTION_FIELD][field] = value;
        assert!(
            receipt::validate(&profile, &yolo_capnp_codec::encode(&injected).unwrap()).is_err()
        );
    }
    let mut wrong = document.clone();
    wrong.schema_version += document.schema_version;
    assert!(receipt::validate(&profile, &yolo_capnp_codec::encode(&wrong).unwrap()).is_err());
    wrong = document;
    wrong.connection.local_origin = yolo_hub_identity::HubId::new();
    assert!(receipt::validate(&profile, &yolo_capnp_codec::encode(&wrong).unwrap()).is_err());
    assert!(receipt::validate(&profile, b"{}").is_err());
}
