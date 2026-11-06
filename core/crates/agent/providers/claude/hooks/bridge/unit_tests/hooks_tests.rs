use super::*;


const SESSION: &str = "123e4567-e89b-42d3-a456-426614174000";
use crate::test_constants::{FUTURE_SESSION_END_REASON, UNRELATED_NOTIFICATION_TYPE};

fn input(event: &str) -> HookInput {
    HookInput {
        session_id: SESSION.to_owned(),
        transcript_path: PathBuf::from("/tmp/transcript.jsonl"),
        cwd: PathBuf::from("/tmp/project"),
        hook_event_name: event.to_owned(),
        source: None,
        new_cwd: None,
        agent_id: None,
        agent_type: None,
        notification_type: None,
        reason: None,
        tool_name: None,
        tool_input: None,
    }
}

#[test]
fn lifecycle_mapping_covers_every_configured_event() {
    let mut start = input(SESSION_START_EVENT);
    start.source = Some(SOURCE_STARTUP.to_owned());
    assert!(matches!(
        lifecycle(start).unwrap().unwrap(),
        ClaudeLifecycleEvent::SessionStart {
            state: ClaudeHookState::WaitingForInput,
            source: ClaudeSessionStartSource::Fresh,
            ..
        }
    ));
    assert!(matches!(
        lifecycle(input(USER_PROMPT_EVENT)).unwrap().unwrap(),
        ClaudeLifecycleEvent::StateChanged {
            state: ClaudeHookState::Running,
            ..
        }
    ));
    assert!(lifecycle(input(STOP_EVENT)).unwrap().is_none());
    assert!(matches!(
        lifecycle(input(SESSION_END_EVENT)).unwrap().unwrap(),
        ClaudeLifecycleEvent::SessionEnd {
            reason: ClaudeSessionEndReason::Other,
            ..
        }
    ));
    assert!(matches!(
        lifecycle(input(CWD_CHANGED_EVENT)).unwrap().unwrap(),
        ClaudeLifecycleEvent::CwdChanged { .. }
    ));
}

#[test]
fn session_end_reasons_are_typed_and_unknown_values_are_retained_as_other() {
    for (provider, expected) in [
        (Some("clear"), ClaudeSessionEndReason::Clear),
        (Some("resume"), ClaudeSessionEndReason::Resume),
        (
            Some(FUTURE_SESSION_END_REASON),
            ClaudeSessionEndReason::Other,
        ),
        (None, ClaudeSessionEndReason::Other),
    ] {
        let mut value = input(SESSION_END_EVENT);
        value.reason = provider.map(str::to_owned);
        let ClaudeLifecycleEvent::SessionEnd { reason, .. } = lifecycle(value).unwrap().unwrap()
        else {
            unreachable!();
        };
        assert_eq!(reason, expected);
    }
}

#[test]
fn session_end_reason_is_required_on_the_versioned_wire() {
    let event = lifecycle(input(SESSION_END_EVENT)).unwrap().unwrap();
    let encoded = serde_json::to_value(&event).unwrap();
    assert_eq!(
        serde_json::from_value::<ClaudeLifecycleEvent>(encoded.clone()).unwrap(),
        event
    );
    let mut missing = encoded;
    missing.as_object_mut().unwrap().remove("reason");
    assert!(serde_json::from_value::<ClaudeLifecycleEvent>(missing).is_err());
}

#[test]
fn stop_does_not_claim_waiting_without_finalization_proof() {
    assert!(lifecycle(input(STOP_EVENT)).unwrap().is_none());
}

#[test]
fn unknown_sources_events_and_noncanonical_sessions_fail_closed() {
    let mut start = input(SESSION_START_EVENT);
    start.source = Some("unknown".to_owned());
    assert!(lifecycle(start).is_err());
    assert!(lifecycle(input("Unknown")).is_err());
    let mut malformed = input(STOP_EVENT);
    malformed.session_id = "not-a-session".to_owned();
    assert!(lifecycle(malformed).is_err());
}

#[test]
fn notifications_track_waiting_and_subagents_are_identifiable() {
    let mut notification = input(NOTIFICATION_EVENT);
    notification.notification_type = Some(WAITING_NOTIFICATION_TYPES[0].to_owned());
    assert!(matches!(
        lifecycle(notification).unwrap().unwrap(),
        ClaudeLifecycleEvent::StateChanged {
            state: ClaudeHookState::WaitingForInput,
            ..
        }
    ));
    let mut unrelated = input(NOTIFICATION_EVENT);
    unrelated.notification_type = Some(UNRELATED_NOTIFICATION_TYPE.to_owned());
    assert!(lifecycle(unrelated).unwrap().is_none());
}

#[test]
fn permission_notification_is_attention_only() {
    let mut notification = input(NOTIFICATION_EVENT);
    notification.notification_type = Some(PERMISSION_NOTIFICATION_TYPE.to_owned());
    assert!(lifecycle(notification).unwrap().is_none());
}

#[test]
fn every_subagent_lifecycle_event_is_dropped_before_delivery() {
    for event in [
        SESSION_START_EVENT,
        SESSION_END_EVENT,
        USER_PROMPT_EVENT,
        STOP_EVENT,
        CWD_CHANGED_EVENT,
        NOTIFICATION_EVENT,
        PRE_TOOL_USE_EVENT,
    ] {
        let mut subagent = input(event);
        subagent.source = Some(SOURCE_STARTUP.to_owned());
        subagent.notification_type = Some(WAITING_NOTIFICATION_TYPES[0].to_owned());
        subagent.agent_id = Some("subagent".to_owned());
        subagent.agent_type = Some("Explore".to_owned());
        assert!(lifecycle(subagent).unwrap().is_none(), "{event}");
    }
}

#[test]
fn top_level_named_agent_type_is_not_mistaken_for_a_subagent() {
    let mut top_level = input(USER_PROMPT_EVENT);
    top_level.agent_type = Some("reviewer".to_owned());
    assert!(matches!(
        lifecycle(top_level).unwrap(),
        Some(ClaudeLifecycleEvent::StateChanged { .. })
    ));
}
