use crate::contract::{QwenHookPayload, QwenSessionStartSource};
use crate::{
    EVENT_INSTRUCTIONS_LOADED, EVENT_MESSAGE_DISPLAY, EVENT_NOTIFICATION, EVENT_POST_TOOL_BATCH,
    EVENT_SESSION_DELETE, EVENT_SESSION_END, EVENT_SESSION_START, EVENT_STOP, EVENT_SUBAGENT_START,
    EVENT_USER_PROMPT_SUBMIT, QwenHookEvent, QwenSessionEndReason,
};
use crate::constants::{
    NOTIFICATION_AUTH_SUCCESS, NOTIFICATION_IDLE_PROMPT, NOTIFICATION_PERMISSION_PROMPT,
    REASON_CLEAR, SOURCE_STARTUP,
};

use crate::test_constants::*;

fn payload(value: serde_json::Value) -> QwenHookPayload {
    serde_json::from_value(value).unwrap()
}

fn base(event: &str) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    for (key, value) in [
        (SESSION_ID, serde_json::json!(TEST_SESSION_ID)),
        (
            TRANSCRIPT_PATH,
            serde_json::json!(TEST_TRANSCRIPT_PATH),
        ),
        (CWD, serde_json::json!(TEST_CWD)),
        (HOOK_EVENT_NAME, serde_json::json!(event)),
        (TIMESTAMP, serde_json::json!(TEST_TIMESTAMP)),
        (PERMISSION_MODE, serde_json::json!(TEST_PERMISSION_MODE)),
    ] {
        object.insert(key.into(), value);
    }
    serde_json::Value::Object(object)
}

#[test]
fn lifecycle_and_idle_events_map_to_canonical_facts() {
    let mut start = base(EVENT_SESSION_START);
    start[SOURCE] = SOURCE_STARTUP.into();
    assert_eq!(
        payload(start).event().unwrap(),
        Some(QwenHookEvent::SessionStart {
            source: QwenSessionStartSource::Startup
        })
    );

    let mut idle = base(EVENT_NOTIFICATION);
    idle[NOTIFICATION_TYPE] = NOTIFICATION_IDLE_PROMPT.into();
    assert_eq!(
        payload(idle).event().unwrap(),
        Some(QwenHookEvent::TurnEnded)
    );

    let mut clear = base(EVENT_SESSION_END);
    clear[REASON] = REASON_CLEAR.into();
    assert_eq!(
        payload(clear).event().unwrap(),
        Some(QwenHookEvent::SessionEnd {
            reason: QwenSessionEndReason::Clear
        })
    );

    let mut deleted = base(EVENT_SESSION_DELETE);
    deleted[REASON] = UNKNOWN_REASON.into();
    assert_eq!(
        payload(deleted).event().unwrap(),
        Some(QwenHookEvent::SessionEnd {
            reason: QwenSessionEndReason::Other
        })
    );
}

#[test]
fn ignored_events_are_not_authority_and_unknown_events_are_rejected() {
    for event in [
        EVENT_STOP,
        EVENT_MESSAGE_DISPLAY,
        EVENT_INSTRUCTIONS_LOADED,
        EVENT_POST_TOOL_BATCH,
    ] {
        assert_eq!(payload(base(event)).event().unwrap(), None);
    }
    let mut auth = base(EVENT_NOTIFICATION);
    auth[NOTIFICATION_TYPE] = NOTIFICATION_AUTH_SUCCESS.into();
    assert_eq!(payload(auth).event().unwrap(), None);
    assert!(payload(base(UNKNOWN_EVENT)).event().is_err());
}

#[test]
fn permission_and_subagent_payloads_require_their_identifiers() {
    let mut permission = base(EVENT_NOTIFICATION);
    permission[NOTIFICATION_TYPE] = NOTIFICATION_PERMISSION_PROMPT.into();
    assert_eq!(
        payload(permission).event().unwrap(),
        Some(QwenHookEvent::NotificationPermission)
    );
    assert!(payload(base(EVENT_SUBAGENT_START)).event().is_err());
    let mut subagent = base(EVENT_SUBAGENT_START);
    subagent[AGENT_ID] = TEST_SUBAGENT.into();
    let payload = payload(subagent);
    assert_eq!(payload.event().unwrap(), Some(QwenHookEvent::SubagentStart));
    assert_eq!(payload.agent_id(), Some(TEST_SUBAGENT));
}

#[test]
fn malformed_envelope_fields_are_rejected() {
    let mut value = base(EVENT_USER_PROMPT_SUBMIT);
    value[CWD] = RELATIVE_CWD.into();
    assert!(payload(value).validate().is_err());
    let mut value = base(EVENT_USER_PROMPT_SUBMIT);
    value[SESSION_ID] = serde_json::Value::String(String::new());
    assert!(payload(value).validate().is_err());
}
