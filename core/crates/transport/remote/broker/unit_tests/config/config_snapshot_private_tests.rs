use super::*;
use crate::remote_broker::BrokerConfig;
use russh::keys::ssh_key::LineEnding;
use russh::keys::{Algorithm, PrivateKey};
use serde_json::{Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use yolo_broker_hub_protocol::HUB_TRANSPORT_FIELD;

const TRUSTED_FILE_MODE: u32 = 0o644;
const EXECUTABLE_FILE_MODE: u32 = 0o755;
const TEST_HOST: &str = "build.example";
const TEST_USER: &str = "builder";
const TEST_HOST_ID: &str = "snapshot-host";
const TEST_PROFILE_AUTHORITY_SHA256: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const TEST_PROGRAM: &str = "/usr/bin/git";
const TEST_READ_ROOT: &str = "/srv/project";
const TEST_RUNTIME: &str = "/opt/yolo";
const TEST_HOME: &str = "/var/lib/yolo";
const TEST_VERSION: &str = "1.0.0";
const KNOWN_HOST_BYTES: &[u8] = b"build.example ssh-ed25519 encoded-key";
const MACHO_ARM64: &[u8] = &[0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1];
const MALFORMED_JSON: &[u8] = b"{";
const INVALID_IDENTITY_PATH: &str = "relative-path-must-not-be-opened";
const EXCESS_IDENTITY_FILES: usize = MAX_IDENTITY_FILES + 1;
const ADVERSARIAL_IDENTITY_FILES: usize = MAX_IDENTITY_FILES * MAX_IDENTITY_FILES;
pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub(super) paths: BrokerPaths,
    pub(super) source: PathBuf,
    known_hosts: PathBuf,
    identity: PathBuf,
    artifact: PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let known_hosts = root.path().join(GENERATION_KNOWN_HOSTS_FILE);
        let identity = root.path().join(identity_name(0));
        let artifact = root.path().join(GENERATION_ARTIFACT_FILE);
        write_mode(&known_hosts, KNOWN_HOST_BYTES, TRUSTED_FILE_MODE);
        write_identity(&identity);
        write_mode(&artifact, MACHO_ARM64, EXECUTABLE_FILE_MODE);
        let source = root.path().join(GENERATION_CONFIG_FILE);
        let paths = BrokerPaths::new(root.path().join("state")).unwrap();
        let fixture = Self {
            _root: root,
            paths,
            source,
            known_hosts,
            identity,
            artifact,
        };
        fixture.write_value(&fixture.value());
        fixture
    }

    fn value(&self) -> Value {
        json!({
            (HUB_TRANSPORT_FIELD): false,
            (VERSION_CONFIG_FIELD): BROKER_CONFIG_VERSION,
            (HOST_ID_CONFIG_FIELD): TEST_HOST_ID,
            (PROFILE_AUTHORITY_SHA256_CONFIG_FIELD): TEST_PROFILE_AUTHORITY_SHA256,
            (ENDPOINT_CONFIG_FIELD): {
                (USER_CONFIG_FIELD): TEST_USER,
                (HOST_CONFIG_FIELD): TEST_HOST
            },
            (KNOWN_HOSTS_CONFIG_FIELD): self.known_hosts,
            (IDENTITIES_CONFIG_FIELD): [self.identity],
            (USE_AGENT_CONFIG_FIELD): false,
            (RUNTIME_CONFIG_FIELD): {
                (EXECUTABLE_CONFIG_FIELD): TEST_RUNTIME,
                (HOME_CONFIG_FIELD): TEST_HOME
            },
            (POLICY_CONFIG_FIELD): {
                (ALLOWED_PROGRAMS_CONFIG_FIELD): [TEST_PROGRAM],
                (READ_ROOTS_CONFIG_FIELD): [TEST_READ_ROOT]
            },
            (ARTIFACT_CONFIG_FIELD): {
                (PATH_CONFIG_FIELD): self.artifact,
                (VERSION_CONFIG_FIELD): TEST_VERSION,
                (ARCHITECTURE_CONFIG_FIELD): ARTIFACT_ARCHITECTURE_AARCH64,
                (SHA256_CONFIG_FIELD): yolo_crypto::sha256_hex(MACHO_ARM64)
            }
        })
    }

    fn write_value(&self, value: &Value) {
        write_mode(
            &self.source,
            &serde_json::to_vec(value).unwrap(),
            PRIVATE_FILE_MODE,
        );
    }
}

fn write_mode(path: &Path, bytes: &[u8], mode: u32) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

fn write_identity(path: &Path) {
    let mut random = russh::keys::key::safe_rng();
    PrivateKey::random(&mut random, Algorithm::Ed25519)
        .unwrap()
        .write_openssh_file(path, LineEnding::LF)
        .unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
}

fn set_identity_count(value: &mut Value, path: &Path, count: usize) {
    value[IDENTITIES_CONFIG_FIELD] = Value::Array((0..count).map(|_| json!(path)).collect());
}

#[test]
fn stages_external_material_and_reuses_verified_generation() {
    let fixture = Fixture::new();
    let first = stage_broker_config(&fixture.source, &fixture.paths).unwrap();
    let directory = first.config_path().parent().unwrap();
    assert_eq!(
        fs::read(directory.join(identity_name(0))).unwrap(),
        fs::read(&fixture.identity).unwrap()
    );
    assert_eq!(
        fs::read(directory.join(GENERATION_ARTIFACT_FILE)).unwrap(),
        MACHO_ARM64
    );
    assert_eq!(
        fs::metadata(directory).unwrap().permissions().mode() & PERMISSION_BITS_MASK,
        IMMUTABLE_DIRECTORY_MODE
    );
    let copied = BrokerConfig::load(first.config_path()).unwrap();
    assert_eq!(copied.revision(), first.revision());
    let second = stage_broker_config(&fixture.source, &fixture.paths).unwrap();
    assert_eq!(second.config_path(), first.config_path());
    assert_eq!(second.revision(), first.revision());
    fs::set_permissions(
        directory,
        fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    assert!(stage_broker_config(&fixture.source, &fixture.paths).is_err());
}

#[test]
fn invalid_sources_and_generation_roots_fail_closed() {
    let fixture = Fixture::new();
    write_mode(&fixture.source, MALFORMED_JSON, PRIVATE_FILE_MODE);
    assert!(stage_broker_config(&fixture.source, &fixture.paths).is_err());
    fixture.write_value(&fixture.value());
    fs::remove_file(&fixture.known_hosts).unwrap();
    assert!(stage_broker_config(&fixture.source, &fixture.paths).is_err());

    let fixture = Fixture::new();
    fs::remove_file(&fixture.identity).unwrap();
    assert!(stage_broker_config(&fixture.source, &fixture.paths).is_err());
    let fixture = Fixture::new();
    fs::remove_file(&fixture.artifact).unwrap();
    assert!(stage_broker_config(&fixture.source, &fixture.paths).is_err());

    let fixture = Fixture::new();
    fs::write(fixture.paths.generations(), b"unsafe").unwrap();
    assert!(stage_broker_config(&fixture.source, &fixture.paths).is_err());
}

#[test]
fn identity_limit_is_enforced_before_identity_file_io() {
    let fixture = Fixture::new();
    let mut value = fixture.value();
    set_identity_count(&mut value, &fixture.identity, MAX_IDENTITY_FILES);
    fixture.write_value(&value);
    stage_broker_config(&fixture.source, &fixture.paths).unwrap();

    for count in [EXCESS_IDENTITY_FILES, ADVERSARIAL_IDENTITY_FILES] {
        let fixture = Fixture::new();
        let mut value = fixture.value();
        set_identity_count(&mut value, Path::new(INVALID_IDENTITY_PATH), count);
        fixture.write_value(&value);
        let error = stage_broker_config(&fixture.source, &fixture.paths).unwrap_err();
        assert!(
            error.to_string().contains(SSH_IDENTITY_COUNT_LIMIT_MESSAGE),
            "{error:#}"
        );
    }
}

#[path = "config_snapshot_optional_tests.rs"]
mod optional;
