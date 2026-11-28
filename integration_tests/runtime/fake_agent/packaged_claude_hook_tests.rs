use crate::fake_agent_constants::*;
use crate::packaged_claude_hook;
use crate::packaged_claude_test_support::*;
use serde_json::Value;

const START_HOOK_FIELD_COUNT: usize = 5;
const NOTIFICATION_HOOK_FIELD_COUNT: usize = 6;

#[test]
fn hook_stdin_serializes_every_provider_event_with_exact_context() {
    let directory = tempfile::tempdir().unwrap();
    let helper = executable(directory.path(), HELPER_FILE_NAME, SUCCESS_SCRIPT);
    let transcript = directory.path().join(TRANSCRIPT_FILE_NAME);
    let cwd = directory.path();

    for &(event, _) in PACKAGED_HOOK_EVENTS {
        let capture = directory.path().join(event);
        let command = command(&helper, Some(&capture));
        let source = (event == PACKAGED_SESSION_START_EVENT).then_some(PACKAGED_START_SOURCE);

        packaged_claude_hook::deliver(&command, event, SESSION_ID, &transcript, cwd, source, None)
            .unwrap();

        let input: Value = serde_json::from_slice(&std::fs::read(&capture).unwrap()).unwrap();
        assert_eq!(input[PACKAGED_HOOK_EVENT_FIELD], event);
        assert_eq!(input[PACKAGED_HOOK_SESSION_FIELD], SESSION_ID);
        assert_eq!(
            input[PACKAGED_HOOK_TRANSCRIPT_FIELD],
            transcript.to_string_lossy().as_ref()
        );
        assert_eq!(
            input[PACKAGED_HOOK_CWD_FIELD],
            cwd.to_string_lossy().as_ref()
        );
        if let Some(source) = source {
            assert_eq!(input[PACKAGED_HOOK_SOURCE_FIELD], source);
            assert_eq!(input.as_object().unwrap().len(), START_HOOK_FIELD_COUNT);
        } else {
            assert!(input.get(PACKAGED_HOOK_SOURCE_FIELD).is_none());
            assert_event_fields(event, &input);
        }
    }
}

fn assert_event_fields(event: &str, input: &Value) {
    use yolo_runtime::claude_hooks as events;
    match event {
        events::CLAUDE_SESSION_END_EVENT => assert_eq!(input["reason"], "other"),
        events::CLAUDE_USER_PROMPT_EVENT => assert!(input["prompt"].is_string()),
        events::CLAUDE_STOP_EVENT => {
            assert_eq!(input[PROVIDER_STOP_ACTIVE_FIELD], false);
            assert!(input[PROVIDER_LAST_MESSAGE_FIELD].is_string());
        }
        events::CLAUDE_CWD_CHANGED_EVENT => assert_eq!(
            input[PACKAGED_HOOK_OLD_CWD_FIELD],
            input[PACKAGED_HOOK_NEW_CWD_FIELD]
        ),
        events::CLAUDE_NOTIFICATION_EVENT => assert!(input["message"].is_string()),
        events::CLAUDE_PRE_TOOL_USE_EVENT => {
            assert!(input[PROVIDER_TOOL_NAME_FIELD].is_string());
            assert!(input[PROVIDER_TOOL_INPUT_FIELD].is_object());
            assert!(input[PROVIDER_TOOL_USE_ID_FIELD].is_string());
        }
        _ => panic!("unexpected event {event}"),
    }
}

#[test]
fn waiting_notification_serializes_its_provider_type() {
    let directory = tempfile::tempdir().unwrap();
    let capture = directory.path().join(PACKAGED_NOTIFICATION_EVENT);
    let command = command(
        &executable(directory.path(), HELPER_FILE_NAME, SUCCESS_SCRIPT),
        Some(&capture),
    );
    packaged_claude_hook::deliver(
        &command,
        PACKAGED_NOTIFICATION_EVENT,
        SESSION_ID,
        &directory.path().join(TRANSCRIPT_FILE_NAME),
        directory.path(),
        None,
        Some(PACKAGED_WAITING_NOTIFICATION_TYPE),
    )
    .unwrap();

    let input: Value = serde_json::from_slice(&std::fs::read(capture).unwrap()).unwrap();
    assert_eq!(
        input[PACKAGED_HOOK_NOTIFICATION_TYPE_FIELD],
        PACKAGED_WAITING_NOTIFICATION_TYPE
    );
    assert_eq!(
        input.as_object().unwrap().len(),
        NOTIFICATION_HOOK_FIELD_COUNT
    );
}

#[test]
fn nonzero_hook_exit_is_reported_as_delivery_failure() {
    let directory = tempfile::tempdir().unwrap();
    let helper = executable(directory.path(), HELPER_FILE_NAME, FAILURE_SCRIPT);
    let command = command(&helper, None);

    let result = packaged_claude_hook::deliver(
        &command,
        PACKAGED_STOP_EVENT,
        SESSION_ID,
        &directory.path().join(TRANSCRIPT_FILE_NAME),
        directory.path(),
        None,
        None,
    );

    assert!(result.is_err());
}
