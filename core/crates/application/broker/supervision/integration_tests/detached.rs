use crate::fixture_constants as fixture;
use crate::{ChildLaunch, CredentialSource, DetachedSpawner};
use std::ffi::OsString;
use std::fs;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use yolo_cli::constants;

#[test]
fn detached_spawner_preserves_non_utf8_config_and_state_paths() {
    let root = tempfile::tempdir().unwrap();
    let script = root.path().join(fixture::ARGUMENT_CAPTURE_SCRIPT_FILE);
    fs::write(&script, argument_capture_script()).unwrap();
    fs::set_permissions(
        &script,
        fs::Permissions::from_mode(fixture::OWNER_EXECUTABLE_MODE),
    )
    .unwrap();
    let config = non_utf8_path(root.path(), fixture::CONFIG_FILE);
    let state_dir = non_utf8_path(root.path(), fixture::STATE_DIRECTORY);
    let launch = ChildLaunch::new(
        config.clone(),
        state_dir.clone(),
        fixture::TEST_CONFIG_REVISION.into(),
        fixture::TEST_STATE_ANCHOR.parse().unwrap(),
        uuid::Uuid::nil().to_string().parse().unwrap(),
        yolo_remote_broker::BrokerLaunchDeadline::capture(
            tokio::time::Instant::now() + fixture::START_TIMEOUT,
        )
        .unwrap(),
    )
    .unwrap();
    let command = crate::server_command::build(
        constants::BROKER_SUPERVISOR_COMMAND,
        &launch,
        CredentialSource::None,
        None,
    )
    .unwrap();
    let role = yolo_launch_contract::ProcessRole::Remote
        .arguments()
        .map(OsString::from);
    assert!(command.arguments.starts_with(&role));
    assert_eq!(
        command.arguments[role.len()],
        OsString::from(constants::BROKER_SUPERVISOR_COMMAND)
    );
    let mut child = yolo_process::spawn_detached_before_deadline(
        &script,
        &command.arguments,
        yolo_process::DetachedStandardError::Null,
        command.deadline,
    )
    .unwrap();
    assert!(child.wait().unwrap().success());

    let config_capture = root.path().join(fixture::CONFIG_CAPTURE_FILE);
    let state_capture = root.path().join(fixture::STATE_CAPTURE_FILE);
    assert_eq!(
        fs::read(config_capture).unwrap(),
        config.as_os_str().as_bytes()
    );
    assert_eq!(
        fs::read(state_capture).unwrap(),
        state_dir.as_os_str().as_bytes()
    );
}

#[test]
fn detached_spawner_forwards_passphrases_only_through_private_stdin() {
    let root = tempfile::tempdir().unwrap();
    let script = root.path().join(fixture::PRIVATE_INPUT_CAPTURE_SCRIPT_FILE);
    let input_capture = root.path().join(fixture::PRIVATE_INPUT_CAPTURE_FILE);
    let metadata_capture = root.path().join(fixture::PRIVATE_ARGUMENT_CAPTURE_FILE);
    fs::write(
        &script,
        private_capture_script(&input_capture, &metadata_capture),
    )
    .unwrap();
    fs::set_permissions(
        &script,
        fs::Permissions::from_mode(fixture::OWNER_EXECUTABLE_MODE),
    )
    .unwrap();
    let encoded = yolo_capnp_codec::encode(&serde_json::json!({
        (fixture::VERSION_FIELD): fixture::TEST_PASSPHRASE_WIRE_VERSION,
        (fixture::BINDING_NONCE_FIELD): fixture::TEST_BINDING_NONCE,
        (fixture::HOST_ID_FIELD): fixture::TEST_HOST_ID,
        (fixture::IDENTITY_CREDENTIAL_REVISION_FIELD): null,
        (fixture::IDENTITY_PATHS_SHA256_FIELD): fixture::TEST_IDENTITY_PATHS_SHA256,
        (fixture::PASSPHRASES_FIELD): [fixture::TEST_IDENTITY_PASSPHRASE],
    }))
    .unwrap();
    let passphrases =
        yolo_remote_broker::IdentityPassphrases::read_checked(encoded.as_slice(), false).unwrap();
    let paths =
        yolo_remote_broker::BrokerPaths::new(root.path().join(fixture::STATE_DIRECTORY)).unwrap();
    let launch = ChildLaunch::new(
        root.path().join(fixture::CONFIG_FILE),
        paths.state_dir().to_owned(),
        fixture::TEST_CONFIG_REVISION.into(),
        paths.state_anchor(),
        uuid::Uuid::nil().to_string().parse().unwrap(),
        yolo_remote_broker::BrokerLaunchDeadline::capture(
            tokio::time::Instant::now() + fixture::START_TIMEOUT,
        )
        .unwrap(),
    )
    .unwrap();
    DetachedSpawner::new(script, passphrases, CredentialSource::PrivateStdin)
        .spawn(&launch, None)
        .err()
        .unwrap();

    let captured: serde_json::Value =
        yolo_capnp_codec::decode(&fs::read(input_capture).unwrap()).unwrap();
    assert_eq!(
        captured,
        yolo_capnp_codec::decode::<serde_json::Value>(&encoded).unwrap()
    );
    let metadata = fs::read_to_string(metadata_capture).unwrap();
    assert!(metadata.contains(constants::IDENTITY_PASSPHRASES_STDIN_ARGUMENT));
    assert!(!metadata.contains(fixture::TEST_IDENTITY_PASSPHRASE));
}

fn non_utf8_path(root: &Path, label: &str) -> PathBuf {
    let mut component = label.as_bytes().to_vec();
    component.push(fixture::NON_UTF8_PATH_BYTE);
    root.join(OsString::from_vec(component))
}

fn argument_capture_script() -> String {
    format!(
        "#!/bin/sh\nwhile [ \"$#\" -gt 0 ]; do\n\
         argument=\"$1\"\nshift\ncase \"$argument\" in\n\
         {}) output={} ;;\n{}) output={} ;;\n*) continue ;;\nesac\n\
         printf '%s' \"$1\" > \"$(dirname \"$0\")/$output\"\nshift\ndone\n",
        constants::CONFIG_ARGUMENT,
        fixture::CONFIG_CAPTURE_FILE,
        constants::STATE_DIRECTORY_ARGUMENT,
        fixture::STATE_CAPTURE_FILE,
    )
}

fn private_capture_script(input: &Path, metadata: &Path) -> String {
    format!(
        "#!/bin/sh\n/usr/bin/printenv > '{}'\nprintf '%s\\n' \"$@\" >> '{}'\n\
         /bin/cat > '{}'\n",
        metadata.display(),
        metadata.display(),
        input.display(),
    )
}
