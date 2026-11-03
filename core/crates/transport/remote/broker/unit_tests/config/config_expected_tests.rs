use super::super::BrokerConfig;
use super::super::constants::BROKER_CONFIG_REVISION_MISMATCH_MESSAGE;
use super::config_tests::ConfigFixture;

const STALE_CONFIG_REVISION: &str = "stale-config-revision";

#[test]
fn expected_revision_accepts_the_loaded_snapshot_identity() {
    let fixture = ConfigFixture::new();
    let path = fixture.write(&fixture.value());
    let expected = BrokerConfig::load(&path).unwrap().revision().to_owned();

    let verified = BrokerConfig::load_expected(&path, &expected).unwrap();

    assert_eq!(verified.revision(), expected);
}

#[test]
fn expected_revision_rejects_a_stale_snapshot_identity() {
    let fixture = ConfigFixture::new();
    let path = fixture.write(&fixture.value());

    let error = BrokerConfig::load_expected(path, STALE_CONFIG_REVISION)
        .err()
        .unwrap();

    assert!(
        error
            .to_string()
            .contains(BROKER_CONFIG_REVISION_MISMATCH_MESSAGE)
    );
}
