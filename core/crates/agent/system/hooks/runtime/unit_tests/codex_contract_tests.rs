use super::test_support::{CODEX_VERSION, envelope, identity};
use super::*;

#[test]
fn codex_mapper_exposes_only_documented_events() {
    for event_name in [
        event_names::SESSION_START,
        event_names::SESSION_END,
        event_names::USER_PROMPT_SUBMIT,
        event_names::PRE_TOOL_USE,
        event_names::PERMISSION_REQUEST,
        event_names::POST_TOOL_USE,
        event_names::PRE_COMPACT,
        event_names::POST_COMPACT,
        event_names::SUBAGENT_START,
        event_names::SUBAGENT_STOP,
        event_names::STOP,
    ] {
        assert!(CodexEventMapper.map(event_name).is_some());
    }
    assert_eq!(
        CodexEventMapper.map(event_names::POST_TOOL_USE_FAILURE),
        None
    );
    assert_eq!(
        CodexEventMapper.map(constants::test_values::UNDOCUMENTED_INTERRUPT_EVENT),
        None
    );
}

#[test]
fn codex_rejects_claude_only_boundaries() {
    for event in [
        EventKind::SessionStart {
            source: SessionStartSource::Fork,
        },
        EventKind::SessionEnd {
            reason: SessionEndReason::Clear,
        },
        EventKind::SessionEnd {
            reason: SessionEndReason::Resume,
        },
    ] {
        assert_codex_rejects(event);
    }
}

#[test]
fn codex_rejects_all_claude_only_event_kinds() {
    for event in [
        EventKind::PostToolUseFailure,
        EventKind::StopFailure,
        EventKind::Notification {
            kind: NotificationKind::Other(String::new()),
        },
        EventKind::PermissionDenied,
        EventKind::CwdChanged,
        EventKind::Elicitation,
        EventKind::ElicitationResult,
    ] {
        assert_codex_rejects(event);
    }
}

fn assert_codex_rejects(event: EventKind) {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let candidate = envelope(&expected, CODEX_VERSION, FIRST_SEQUENCE, event);
    assert_eq!(
        reducer.apply(candidate),
        Err(HookViolation::UnsupportedEvent)
    );
    assert_eq!(
        reducer.state().instrumentation,
        InstrumentationState::Unavailable
    );
}
