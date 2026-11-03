use super::constants::{event_names, field_names, permission_modes, test_values, values};

#[test]
fn every_pinned_event_fixture_decodes() {
    for event in event_names::ALL {
        assert!(
            super::wire_test_support::decode_value(super::wire_test_support::input_value(event))
                .is_ok(),
            "{event}"
        );
    }
}

#[test]
fn every_required_field_is_rejected_when_missing() {
    for event in event_names::ALL {
        let valid = super::wire_test_support::input_value(event);
        let fields: Vec<_> = valid.as_object().unwrap().keys().cloned().collect();
        for field in fields {
            let mut missing = valid.clone();
            missing.as_object_mut().unwrap().remove(&field);
            assert!(
                super::wire_test_support::decode_value(missing).is_err(),
                "{event}: {field}"
            );
        }
    }
}

#[test]
fn every_event_rejects_unknown_and_cross_event_fields() {
    for event in event_names::ALL {
        assert_extra_rejected(event, test_values::UNKNOWN_FIELD);
    }
    for (event, field) in cross_event_fields() {
        assert_extra_rejected(event, field);
    }
}

#[test]
fn provider_cannot_supply_an_event_identity() {
    for event in event_names::ALL {
        assert_extra_rejected(event, test_values::PROVIDER_EVENT_ID_FIELD);
    }
}

#[test]
fn event_discriminator_and_value_domains_are_exact() {
    let unknown = super::wire_test_support::input_value(test_values::UNKNOWN_EVENT);
    assert!(super::wire_test_support::decode_value(unknown).is_err());
    let mut interrupt = super::wire_test_support::input_value(event_names::STOP);
    interrupt[field_names::EVENT_NAME] =
        serde_json::json!(test_values::UNDOCUMENTED_INTERRUPT_EVENT);
    assert!(super::wire_test_support::decode_value(interrupt).is_err());

    for mode in permission_modes::ALL {
        let mut raw = super::wire_test_support::input_value(event_names::STOP);
        raw[field_names::PERMISSION_MODE] = serde_json::json!(mode);
        assert!(
            super::wire_test_support::decode_value(raw).is_ok(),
            "{mode}"
        );
    }
    let mut invalid = super::wire_test_support::input_value(event_names::STOP);
    invalid[field_names::PERMISSION_MODE] = serde_json::json!(test_values::UNKNOWN_PERMISSION_MODE);
    assert!(super::wire_test_support::decode_value(invalid).is_err());

    for trigger in values::COMPACT_TRIGGERS {
        let mut raw = super::wire_test_support::input_value(event_names::PRE_COMPACT);
        raw[field_names::TRIGGER] = serde_json::json!(trigger);
        assert!(
            super::wire_test_support::decode_value(raw).is_ok(),
            "{trigger}"
        );
    }
}

#[test]
fn required_nullable_and_untyped_fields_keep_their_wire_types() {
    let mut invalid = super::wire_test_support::input_value(event_names::SUBAGENT_STOP);
    invalid[field_names::AGENT_TRANSCRIPT_PATH] = serde_json::Value::Bool(false);
    assert!(super::wire_test_support::decode_value(invalid).is_err());

    let mut null_tool = super::wire_test_support::input_value(event_names::PRE_TOOL_USE);
    null_tool[field_names::TOOL_INPUT] = serde_json::Value::Null;
    assert!(super::wire_test_support::decode_value(null_tool).is_ok());
}

#[test]
fn optional_agent_fields_reject_explicit_null() {
    for event in [
        event_names::USER_PROMPT_SUBMIT,
        event_names::PRE_TOOL_USE,
        event_names::PERMISSION_REQUEST,
        event_names::POST_TOOL_USE,
        event_names::PRE_COMPACT,
        event_names::POST_COMPACT,
    ] {
        for field in [field_names::AGENT_ID, field_names::AGENT_TYPE] {
            let mut raw = super::wire_test_support::input_value(event);
            assert!(super::wire_test_support::decode_value(raw.clone()).is_ok());
            raw[field] = serde_json::Value::Null;
            assert!(
                super::wire_test_support::decode_value(raw).is_err(),
                "{event}: {field}"
            );
        }
    }
}

fn assert_extra_rejected(event: &str, field: &str) {
    let mut raw = super::wire_test_support::input_value(event);
    raw[field] = serde_json::json!(test_values::UNKNOWN_FIELD);
    assert!(
        super::wire_test_support::decode_value(raw).is_err(),
        "{event}: {field}"
    );
}

fn cross_event_fields() -> &'static [(&'static str, &'static str)] {
    &[
        (event_names::SESSION_START, field_names::TURN_ID),
        (event_names::SESSION_END, field_names::MODEL),
        (event_names::USER_PROMPT_SUBMIT, field_names::TOOL_NAME),
        (event_names::PRE_TOOL_USE, field_names::TOOL_RESPONSE),
        (event_names::PERMISSION_REQUEST, field_names::TOOL_USE_ID),
        (event_names::POST_TOOL_USE, field_names::PROMPT),
        (event_names::PRE_COMPACT, field_names::PERMISSION_MODE),
        (event_names::POST_COMPACT, field_names::SOURCE),
        (event_names::SUBAGENT_START, field_names::STOP_HOOK_ACTIVE),
        (event_names::SUBAGENT_STOP, field_names::PROMPT),
        (event_names::STOP, field_names::AGENT_ID),
    ]
}
