use std::path::PathBuf;
use uuid::Uuid;
use yolo_hub_ssh_profile_protocol::*;
use yolo_hub_ssh_profile_protocol_test_support::{constants::*, *};

#[test]
fn broker_paths_are_bounded_lexical_references_and_validation_never_requires_files() {
    let mut value = profile(0);
    value.broker_state = VALID_UNCREATED_PATH.into();
    value.validate().unwrap();
    for path in [
        "/",
        "relative",
        "/private/../broker",
        "/private/./broker",
        "/private//broker",
        "/private/broker/",
        "/private/broker\0",
        "/private/broker\n",
    ] {
        value.broker_state = path.into();
        assert!(value.validate().is_err());
    }
    value.broker_state = PathBuf::from(format!(
        "/{BROKER_PREFIX}{}",
        "x".repeat(MAX_BROKER_STATE_PATH_BYTES)
    ));
    assert!(value.validate().is_err());
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        value.broker_state = std::ffi::OsString::from_vec(vec![u8::MAX]).into();
        assert!(value.validate().is_err());
    }
    for digest in [
        String::new(),
        INVALID_REVISION.into(),
        CONFIG_REVISION.to_uppercase(),
    ] {
        let mut value = profile(0);
        value.config_revision = digest;
        assert!(value.validate().is_err());
    }
}

#[test]
fn configuration_enforces_capacity_schema_and_unique_profile_and_broker_targets() {
    SshProfileConfiguration::default().validate().unwrap();
    let mut value = SshProfileConfiguration {
        profiles: (0..MAX_SSH_PROFILES).map(profile).collect(),
        ..Default::default()
    };
    value.validate().unwrap();
    value.profiles.push(profile(MAX_SSH_PROFILES));
    assert!(value.validate().is_err());
    let mut value = configuration();
    value.schema_version += 1;
    assert!(value.validate().is_err());
    let mut value = configuration();
    value.profiles.push(profile(0));
    assert!(value.validate().is_err());
    let mut value = configuration();
    let mut second = profile(1);
    second.broker_state = value.profiles[0].broker_state.clone();
    value.profiles.push(second);
    assert!(value.validate().is_err());
    let mut value = configuration();
    value.profiles[0].config_revision = INVALID_REVISION.into();
    assert!(value.validate().is_err());
}

#[test]
fn revision_and_request_identity_are_required_for_state_and_changes() {
    let current = state();
    current.validate().unwrap();
    let mut invalid = current.clone();
    invalid.revision = 0;
    assert!(invalid.validate().is_err());
    let mut invalid = current.clone();
    invalid.last_change = Uuid::nil();
    assert!(invalid.validate().is_err());
    let mut invalid = current.clone();
    invalid.configuration.schema_version += 1;
    assert!(invalid.validate().is_err());
    let update = change(&current, configuration());
    update.validate().unwrap();
    let mut invalid = update.clone();
    invalid.request_id = Uuid::nil();
    assert!(invalid.validate().is_err());
    let mut invalid = update.clone();
    invalid.expected_revision = 0;
    assert!(invalid.validate().is_err());
    let mut invalid = update;
    invalid.configuration.schema_version += 1;
    assert!(invalid.validate().is_err());
}
