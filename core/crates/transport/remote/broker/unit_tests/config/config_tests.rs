use super::super::constants::*;
use super::super::*;
use super::support::*;
use russh::keys::ssh_key::LineEnding;
use russh::keys::{Algorithm, PrivateKey};
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use yolo_broker_hub_protocol::HUB_TRANSPORT_FIELD;

const OWNER_PUBLIC_DIRECTORY_MODE: u32 = 0o755;

pub(super) struct ConfigFixture {
    pub root: TempDir,
    pub known_hosts: PathBuf,
    pub identity: PathBuf,
}

impl ConfigFixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let known_hosts = root
            .path()
            .join(super::super::constants::KNOWN_HOSTS_CONFIG_FIELD);
        let identity = root.path().join("identity");
        write_mode(
            &known_hosts,
            b"build.example ssh-ed25519 encoded-key",
            0o644,
        );
        write_identity(&identity);
        Self {
            root,
            known_hosts,
            identity,
        }
    }

    pub fn value(&self) -> Value {
        json!({
            (HUB_TRANSPORT_FIELD): false,
            (VERSION_CONFIG_FIELD): 1,
            (HOST_ID_CONFIG_FIELD): HOST_ID,
            (PROFILE_AUTHORITY_SHA256_CONFIG_FIELD): PROFILE_AUTHORITY_SHA256,
            (ENDPOINT_CONFIG_FIELD): {
                (USER_CONFIG_FIELD): "builder",
                (HOST_CONFIG_FIELD): "build.example"
            },
            (KNOWN_HOSTS_CONFIG_FIELD): self.known_hosts,
            (IDENTITIES_CONFIG_FIELD): [self.identity],
            (USE_AGENT_CONFIG_FIELD): false,
            (RUNTIME_CONFIG_FIELD): {
                (EXECUTABLE_CONFIG_FIELD): "/opt/yolo",
                (HOME_CONFIG_FIELD): "/var/lib/yolo"
            },
            (POLICY_CONFIG_FIELD): {
                (ALLOWED_PROGRAMS_CONFIG_FIELD): [ALLOWED_PROGRAM],
                (READ_ROOTS_CONFIG_FIELD): [READ_ROOT]
            }
        })
    }

    pub fn write(&self, value: &Value) -> PathBuf {
        let path = self
            .root
            .path()
            .join(format!("config-{}.json", uuid::Uuid::new_v4()));
        write_mode(&path, serde_json::to_vec(value).unwrap().as_slice(), 0o600);
        path
    }

    pub fn rotate_identity(&self) {
        write_identity(&self.identity);
    }
}

#[test]
fn missing_hub_transport_field_is_rejected_as_an_unsupported_config_shape() {
    let fixture = ConfigFixture::new();
    let mut value = fixture.value();
    value.as_object_mut().unwrap().remove(HUB_TRANSPORT_FIELD);
    assert!(BrokerConfig::load(fixture.write(&value)).is_err());
}

fn write_identity(path: &Path) {
    let mut random = russh::keys::key::safe_rng();
    PrivateKey::random(&mut random, Algorithm::Ed25519)
        .unwrap()
        .write_openssh_file(path, LineEnding::LF)
        .unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
}

pub(super) fn write_mode(path: &Path, data: &[u8], mode: u32) {
    fs::write(path, data).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

#[test]
fn valid_config_resolves_all_scoped_policies_and_paths() {
    let fixture = ConfigFixture::new();
    let identity_material = fs::read(&fixture.identity).unwrap();
    let config = BrokerConfig::load(fixture.write(&fixture.value())).unwrap();
    assert_eq!(config.host_id(), HOST_ID);
    assert_eq!(config.endpoint.host(), "build.example");
    assert_eq!(config.identities[0].path(), fixture.identity);
    assert_eq!(
        config.identities[0].material_for_broker_test(),
        Some(identity_material.as_slice())
    );
    assert!(config.agent_socket.is_none());
    assert_eq!(config.runtime().home(), Path::new("/var/lib/yolo"));
}

#[test]
fn private_config_and_identity_permissions_are_fail_closed() {
    let fixture = ConfigFixture::new();
    let config_path = fixture.write(&fixture.value());
    fs::set_permissions(&config_path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(
        BrokerConfig::load(&config_path)
            .err()
            .unwrap()
            .to_string()
            .contains("permissions")
    );

    fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::set_permissions(&fixture.identity, fs::Permissions::from_mode(0o640)).unwrap();
    assert!(BrokerConfig::load(&config_path).is_err());
}

#[test]
fn config_rejects_symlinks_unknown_fields_and_unsafe_identifiers() {
    let fixture = ConfigFixture::new();
    let real = fixture.write(&fixture.value());
    let link = fixture.root.path().join("config-link.json");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert!(BrokerConfig::load(link).is_err());

    let mut value = fixture.value();
    value["unexpected"] = json!(true);
    assert!(BrokerConfig::load(fixture.write(&value)).is_err());
    let mut unsafe_id = fixture.value();
    unsafe_id[HOST_ID_CONFIG_FIELD] = json!("../../escape");
    assert!(BrokerConfig::load(fixture.write(&unsafe_id)).is_err());
}

#[test]
fn paths_create_and_normalize_private_state_but_reject_unsafe_locations() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join("state");
    let paths = BrokerPaths::new(&state).unwrap();
    assert_eq!(paths.state_dir(), state);
    assert_eq!(paths.runtime_socket().file_name().unwrap(), "runtime.sock");
    assert_eq!(
        fs::metadata(&state).unwrap().permissions().mode() & PERMISSION_BITS_MASK,
        PRIVATE_DIRECTORY_MODE
    );
    assert!(BrokerPaths::new("relative/state").is_err());
    assert!(BrokerPaths::new(Path::new("/")).is_err());

    let public = root.path().join("public");
    fs::create_dir(&public).unwrap();
    fs::set_permissions(
        &public,
        fs::Permissions::from_mode(OWNER_PUBLIC_DIRECTORY_MODE),
    )
    .unwrap();
    assert!(BrokerPaths::new(&public).is_ok());
    assert_eq!(
        fs::metadata(public).unwrap().permissions().mode() & PERMISSION_BITS_MASK,
        PRIVATE_DIRECTORY_MODE
    );
}
