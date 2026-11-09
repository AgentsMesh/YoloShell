use super::*;

mod constants;
mod validation;

#[test]
fn limits_are_nonzero_and_hard_bounded() {
    assert!(ConversationLimits::default().validate().is_ok());
    assert!(
        ConversationLimits {
            entries: MAX_ENTRY_LIMIT,
            bytes: MAX_BYTE_LIMIT
        }
        .validate()
        .is_ok()
    );
    for limits in [
        ConversationLimits {
            entries: 0,
            ..ConversationLimits::default()
        },
        ConversationLimits {
            entries: MAX_ENTRY_LIMIT + 1,
            ..ConversationLimits::default()
        },
        ConversationLimits {
            bytes: 0,
            ..ConversationLimits::default()
        },
        ConversationLimits {
            bytes: MAX_BYTE_LIMIT + 1,
            ..ConversationLimits::default()
        },
    ] {
        assert_eq!(limits.validate(), Err(ConversationError::InvalidRequest));
    }
}

#[test]
fn wire_rejects_unknown_fields_and_invalid_identity() {
    assert!(
        serde_json::from_value::<ConversationLimits>(serde_json::json!({
            "entries": 1, "bytes": 20, "fallback_to_terminal": true,
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<ConversationAddress>(serde_json::json!({
            (constants::HUB_ID_FIELD): "00000000-0000-0000-0000-000000000000", (constants::CONVERSATION_ID_FIELD): vec![0; 32],
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<ConversationPart>(serde_json::json!({
            "kind": constants::IMAGE_UNAVAILABLE_KIND, (constants::MEDIA_TYPE_FIELD): null, "local_path": "/etc/passwd",
        }))
        .is_err()
    );
}

#[test]
fn provider_tokens_are_open_preserved_and_bounded() {
    for token in [
        CODEX_PROVIDER_TOKEN,
        CLAUDE_PROVIDER_TOKEN,
        QWEN_PROVIDER_TOKEN,
        GEMINI_PROVIDER_TOKEN,
        GROK_PROVIDER_TOKEN,
        "future_provider",
    ] {
        let value = ConversationProvider::new(token).unwrap();
        assert_eq!(value.as_str(), token);
        assert_eq!(
            serde_json::to_value(&value).unwrap(),
            serde_json::json!(token)
        );
        assert_eq!(
            serde_json::from_value::<ConversationProvider>(serde_json::json!(token)).unwrap(),
            value
        );
    }
    for invalid in ["", "Qwen", "qwen code", "qwen/provider"] {
        assert!(ConversationProvider::new(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn failures_are_machine_readable_and_do_not_carry_host_paths() {
    for error in [
        ConversationError::InvalidRequest,
        ConversationError::BindingChanged,
        ConversationError::UnsupportedProvider,
        ConversationError::EvidenceUnavailable,
        ConversationError::EvidenceChanged,
        ConversationError::HistoryChanged,
        ConversationError::HistoryLimitExceeded,
        ConversationError::ReaderBusy,
    ] {
        assert!(!error.to_string().is_empty());
        assert_eq!(
            serde_json::from_str::<ConversationError>(&serde_json::to_string(&error).unwrap())
                .unwrap(),
            error
        );
    }
}
