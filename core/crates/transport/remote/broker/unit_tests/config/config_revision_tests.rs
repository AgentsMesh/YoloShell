use super::super::constants::*;
use super::super::*;
use super::config_tests::{ConfigFixture, write_mode};
use super::support::ALLOWED_PROGRAM;
use serde_json::json;

const ROTATED_KNOWN_HOST: &[u8] = b"build.example ssh-ed25519 rotated-key";
const SECOND_ALLOWED_PROGRAM: &str = "/usr/bin/env";

#[test]
fn revision_is_deterministic_and_tracks_external_security_inputs() {
    let fixture = ConfigFixture::new();
    let path = fixture.write(&fixture.value());
    let first = BrokerConfig::load(&path).unwrap().revision().to_owned();
    assert_eq!(BrokerConfig::load(&path).unwrap().revision(), first);

    write_mode(&fixture.known_hosts, ROTATED_KNOWN_HOST, 0o644);
    let known_hosts = BrokerConfig::load(&path).unwrap().revision().to_owned();
    assert_ne!(known_hosts, first);

    fixture.rotate_identity();
    let identity = BrokerConfig::load(&path).unwrap().revision().to_owned();
    assert_ne!(identity, known_hosts);
}

#[test]
fn revision_tracks_inline_policy_and_artifact_bytes() {
    const MACHO_ARM64: &[u8] = &[0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1];
    let fixture = ConfigFixture::new();
    let baseline = BrokerConfig::load(fixture.write(&fixture.value()))
        .unwrap()
        .revision()
        .to_owned();
    let mut policy = fixture.value();
    policy[POLICY_CONFIG_FIELD][ALLOWED_PROGRAMS_CONFIG_FIELD] =
        json!([ALLOWED_PROGRAM, SECOND_ALLOWED_PROGRAM]);
    assert_ne!(
        BrokerConfig::load(fixture.write(&policy))
            .unwrap()
            .revision(),
        baseline
    );

    let artifact = fixture.root.path().join("revision-yolo-host");
    write_mode(&artifact, MACHO_ARM64, 0o755);
    let mut value = fixture.value();
    value[ARTIFACT_CONFIG_FIELD] = json!({
        (PATH_CONFIG_FIELD): artifact,
        (VERSION_CONFIG_FIELD): "1.0.0",
        (ARCHITECTURE_CONFIG_FIELD): ARTIFACT_ARCHITECTURE_AARCH64,
        (SHA256_CONFIG_FIELD): yolo_crypto::sha256_hex(MACHO_ARM64),
    });
    let first = BrokerConfig::load(fixture.write(&value)).unwrap();
    assert!(first.artifact.is_some());
    assert_ne!(first.revision(), baseline);
}

#[test]
fn config_helpers_cover_default_agent_and_host_id_boundaries() {
    assert!(super::super::config::default_use_agent());
    assert!(super::super::config::resolve_agent_socket(true, None).is_ok());
    assert!(super::super::config::validate_host_id("").is_err());
    let oversized = "x".repeat(super::super::constants::MAX_HOST_ID_BYTES + 1);
    assert!(super::super::config::validate_host_id(&oversized).is_err());
}
