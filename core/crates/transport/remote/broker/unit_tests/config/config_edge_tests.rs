use super::super::constants::*;
use super::super::*;
use super::config_tests::{ConfigFixture, write_mode};
use serde_json::json;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tokio::net::UnixListener;

const INVALID_IDENTITY_BYTES: &[u8] = b"not-an-openssh-private-key";

#[test]
fn trusted_inputs_reject_writable_known_hosts_and_non_file_identities() {
    let fixture = ConfigFixture::new();
    let config = fixture.write(&fixture.value());
    fs::set_permissions(&fixture.known_hosts, fs::Permissions::from_mode(0o666)).unwrap();
    assert!(BrokerConfig::load(&config).is_err());

    let fixture = ConfigFixture::new();
    fs::remove_file(&fixture.identity).unwrap();
    fs::create_dir(&fixture.identity).unwrap();
    assert!(BrokerConfig::load(fixture.write(&fixture.value())).is_err());

    let fixture = ConfigFixture::new();
    write_mode(&fixture.identity, INVALID_IDENTITY_BYTES, PRIVATE_FILE_MODE);
    assert!(BrokerConfig::load(fixture.write(&fixture.value())).is_err());
}

#[tokio::test]
async fn explicit_agent_must_be_an_owned_socket_and_requires_agent_enabled() {
    let fixture = ConfigFixture::new();
    let regular = fixture.root.path().join("agent");
    write_mode(&regular, b"not-a-socket", 0o600);
    let mut value = fixture.value();
    value[USE_AGENT_CONFIG_FIELD] = json!(true);
    value[AGENT_SOCKET_CONFIG_FIELD] = json!(regular);
    assert!(BrokerConfig::load(fixture.write(&value)).is_err());

    let socket = fixture.root.path().join("agent.sock");
    let _listener = UnixListener::bind(&socket).unwrap();
    value[AGENT_SOCKET_CONFIG_FIELD] = json!(socket);
    let loaded = BrokerConfig::load(fixture.write(&value)).unwrap();
    assert_eq!(loaded.agent_socket.as_deref(), Some(socket.as_path()));

    value[USE_AGENT_CONFIG_FIELD] = json!(false);
    assert!(BrokerConfig::load(fixture.write(&value)).is_err());
}

#[test]
fn files_are_size_checked_before_unbounded_allocation() {
    let fixture = ConfigFixture::new();
    let config = fixture.root.path().join("oversized.json");
    let file = fs::File::create(&config).unwrap();
    file.set_len((super::super::constants::MAX_CONFIG_FILE_BYTES + 1) as u64)
        .unwrap();
    fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(
        BrokerConfig::load(config)
            .err()
            .unwrap()
            .to_string()
            .contains("size limit")
    );
}

#[test]
fn artifact_is_integrity_checked_and_capped() {
    const MACHO_ARM64: &[u8] = &[0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1];
    const MACHO_X86_64: &[u8] = &[0xcf, 0xfa, 0xed, 0xfe, 0x07, 0, 0, 1];
    const DIGEST: &str = "6eca5cc86a53c80d74e777fffd48ee46b98f06a8b448d18106af49ac0862b875";
    const INVALID_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    let fixture = ConfigFixture::new();
    let artifact = fixture.root.path().join("yolo-host");
    write_mode(&artifact, MACHO_ARM64, 0o755);
    let mut value = fixture.value();
    value[ARTIFACT_CONFIG_FIELD] = json!({
        (PATH_CONFIG_FIELD): artifact,
        (VERSION_CONFIG_FIELD): "1.0.0",
        (ARCHITECTURE_CONFIG_FIELD): ARTIFACT_ARCHITECTURE_AARCH64,
        (SHA256_CONFIG_FIELD): DIGEST
    });
    assert!(
        BrokerConfig::load(fixture.write(&value))
            .unwrap()
            .artifact
            .is_some()
    );

    value[ARTIFACT_CONFIG_FIELD][SHA256_CONFIG_FIELD] = json!(INVALID_DIGEST);
    assert!(BrokerConfig::load(fixture.write(&value)).is_err());

    write_mode(&artifact, MACHO_X86_64, 0o755);
    value[ARTIFACT_CONFIG_FIELD][ARCHITECTURE_CONFIG_FIELD] = json!(ARTIFACT_ARCHITECTURE_X86_64);
    value[ARTIFACT_CONFIG_FIELD][SHA256_CONFIG_FIELD] =
        json!(yolo_crypto::sha256_hex(MACHO_X86_64));
    assert_eq!(
        BrokerConfig::load(fixture.write(&value))
            .unwrap()
            .artifact
            .unwrap()
            .architecture(),
        crate::remote::RemoteArchitecture::X86_64
    );

    let oversized = fixture.root.path().join("oversized-host");
    let file = fs::File::create(&oversized).unwrap();
    file.set_len((super::super::constants::MAX_ARTIFACT_FILE_BYTES + 1) as u64)
        .unwrap();
    fs::set_permissions(&oversized, fs::Permissions::from_mode(0o755)).unwrap();
    value[ARTIFACT_CONFIG_FIELD][PATH_CONFIG_FIELD] = json!(oversized);
    assert!(BrokerConfig::load(fixture.write(&value)).is_err());
}

#[test]
fn invalid_versions_endpoints_and_excess_identities_are_rejected() {
    let fixture = ConfigFixture::new();
    let mut value = fixture.value();
    value[VERSION_CONFIG_FIELD] = json!(99);
    assert!(BrokerConfig::load(fixture.write(&value)).is_err());
    value = fixture.value();
    value[ENDPOINT_CONFIG_FIELD][PORT_CONFIG_FIELD] = json!(0);
    assert!(BrokerConfig::load(fixture.write(&value)).is_err());
    value = fixture.value();
    value[IDENTITIES_CONFIG_FIELD] = json!(vec![fixture.identity.clone(); 33]);
    assert!(BrokerConfig::load(fixture.write(&value)).is_err());
}

#[test]
fn invalid_known_hosts_and_nested_policies_fail_at_their_boundaries() {
    const NON_UTF8: &[u8] = &[0xff];
    const MALFORMED_KNOWN_HOSTS: &[u8] = b"malformed";
    const INVALID_ENDPOINT_HOST: &str = "invalid/host";
    const INVALID_EXECUTABLE: &str = "relative/yolo-host";
    const INVALID_PROGRAM: &str = "relative/git";
    let fixture = ConfigFixture::new();
    let config_path = fixture.write(&fixture.value());

    write_mode(&fixture.known_hosts, NON_UTF8, 0o644);
    assert!(BrokerConfig::load(&config_path).is_err());
    write_mode(&fixture.known_hosts, MALFORMED_KNOWN_HOSTS, 0o644);
    assert!(BrokerConfig::load(&config_path).is_err());

    let mut value = fixture.value();
    value[ENDPOINT_CONFIG_FIELD][HOST_CONFIG_FIELD] = json!(INVALID_ENDPOINT_HOST);
    assert!(BrokerConfig::load(fixture.write(&value)).is_err());

    let mut value = fixture.value();
    value[RUNTIME_CONFIG_FIELD][EXECUTABLE_CONFIG_FIELD] = json!(INVALID_EXECUTABLE);
    assert!(BrokerConfig::load(fixture.write(&value)).is_err());

    let mut value = fixture.value();
    value[POLICY_CONFIG_FIELD][ALLOWED_PROGRAMS_CONFIG_FIELD] = json!([INVALID_PROGRAM]);
    assert!(BrokerConfig::load(fixture.write(&value)).is_err());

    let mut value = fixture.value();
    value[POLICY_CONFIG_FIELD][READ_ROOTS_CONFIG_FIELD] = json!(["/"]);
    assert!(BrokerConfig::load(fixture.write(&value)).is_err());
}
