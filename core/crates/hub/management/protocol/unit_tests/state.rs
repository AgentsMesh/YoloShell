use super::fixtures::{constants::*, *};

#[test]
fn state_roundtrips_its_identity_and_applied_change_without_protocol_local_metadata() {
    let state = state();
    state.validate().unwrap();
    roundtrip(&state);
}

#[test]
fn state_invalid_schema_revision_request_and_identity_configuration_fail_closed() {
    let original = state();
    let mutations: &[fn(&mut crate::ConfigurationState)] = &[
        |s| s.schema_version += SCHEMA_STEP,
        |s| s.revision = 0,
        |s| s.last_change = uuid::Uuid::nil(),
        |s| s.configuration.peers[0].identity = s.identity,
    ];
    for mutate in mutations {
        let mut invalid = original.clone();
        mutate(&mut invalid);
        assert!(invalid.validate().is_err());
        assert!(
            serde_json::from_value::<crate::ConfigurationState>(
                serde_json::to_value(invalid).unwrap()
            )
            .is_err()
        );
    }
    reject(&original, UNKNOWN_FIELD, UNKNOWN_VALUE.into());
    reject(&original, SCHEMA_FIELD, u16::MAX.into());
    reject(&original, REVISION_FIELD, u64::MIN.into());
    for value in [NIL_ID, MALFORMED_ID, UPPER_ID, COMPACT_ID] {
        reject(&original, LAST_CHANGE_FIELD, value.into());
    }
}
