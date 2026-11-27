use super::super::*;
use super::config_tests::ConfigFixture;
use std::os::unix::fs::PermissionsExt;

use yolo_broker_hub_protocol::HUB_TRANSPORT_FIELD as HUB_TRANSPORT;
const ARTIFACT: &str = "artifact";
const PRIVATE: u32 = 0o700;
const PUBLIC: u32 = 0o755;
const STATE: &str = "hub-broker";
const MISSING: &str = "missing";

#[test]
fn explicit_hub_mode_changes_revision_and_survives_immutable_generation_snapshot() {
    let fixture = ConfigFixture::new();
    let mut raw = fixture.value();
    let host = BrokerConfig::load(fixture.write(&raw)).unwrap();
    assert!(!host.hub_transport());
    raw[HUB_TRANSPORT] = serde_json::Value::Bool(true);
    let path = fixture.write(&raw);
    let hub = BrokerConfig::load(&path).unwrap();
    assert!(hub.hub_transport());
    assert_ne!(host.revision(), hub.revision());
    let paths = BrokerPaths::new(fixture.root.path().join(STATE)).unwrap();
    let generation = stage_broker_config(&path, &paths).unwrap();
    assert!(generation.hub_transport());
    assert!(
        BrokerConfig::load_expected(generation.config_path(), generation.revision())
            .unwrap()
            .hub_transport()
    );
    raw[ARTIFACT] = serde_json::json!({"path":"/missing/forbidden", "version":"1.0.0", "architecture":"aarch64", "sha256":yolo_broker_hub_test_support::HASH});
    let error = BrokerConfig::load(fixture.write(&raw)).err().unwrap();
    assert_eq!(
        error.downcast_ref::<BrokerHubError>(),
        Some(&BrokerHubError::Artifact)
    );
}

#[test]
fn existing_broker_path_open_neither_creates_nor_repairs_directories() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join(MISSING);
    assert!(BrokerPaths::open_existing(&missing).is_err());
    assert!(!missing.exists());
    let path = root.path().join(STATE);
    std::fs::create_dir(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(PUBLIC)).unwrap();
    assert!(BrokerPaths::open_existing(&path).is_err());
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & PUBLIC,
        PUBLIC
    );
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(PRIVATE)).unwrap();
    drop(BrokerPaths::new(&path).unwrap());
    let paths = BrokerPaths::open_existing(&path).unwrap();
    paths.verify_state().unwrap();
    assert_eq!(std::fs::read_dir(&path).unwrap().count(), usize::default());
    assert!(BrokerPaths::open_existing(STATE).is_err());
}
