use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::tempdir;
use yolo_local_lifecycle::validate_state;
use yolo_local_profile::LocalProfile;

fn profile(directory: &Path) -> LocalProfile {
    fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).unwrap();
    let path = directory.join("profile.json");
    fs::write(
        &path,
        format!(
            "{{\"schema_version\":1,\"state\":\"{}\",\"host_home\":\"{}\"}}\n",
            directory.join("state").display(),
            directory.join("home").display()
        ),
    )
    .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    LocalProfile::load(&path).unwrap()
}

#[test]
fn absent_state_directory_needs_no_contract() {
    let directory = tempdir().unwrap();
    let profile = profile(directory.path());
    validate_state(&profile).unwrap();
}

#[test]
fn preflight_accepts_the_installed_contract_and_rejects_a_newer_one() {
    let directory = tempdir().unwrap();
    let profile = profile(directory.path());
    drop(yolo_hub_state::HubState::open(&profile.state, &profile.host_home).unwrap());
    validate_state(&profile).unwrap();

    let contract = profile.state.join(yolo_hub_state::STATE_CONTRACT_FILE);
    let current = yolo_hub_rpc_protocol::HUB_PROTOCOL_VERSION;
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(&contract).unwrap()).unwrap();
    assert_eq!(document["hub_protocol"], serde_json::json!(current));
    document["hub_protocol"] = serde_json::json!(u16::MAX);
    fs::write(&contract, serde_json::to_vec(&document).unwrap()).unwrap();
    assert!(validate_state(&profile).is_err());
}
