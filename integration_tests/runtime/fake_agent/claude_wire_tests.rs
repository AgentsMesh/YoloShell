use super::claude_wire::*;
use super::claude_wire_path::transcript;
use super::fake_agent_constants::*;
use serde_json::Value;
use std::path::{Path, PathBuf};

const YOLO_SESSION_ID: &str = "11111111-1111-4111-8111-111111111111";
const RUNTIME_EPOCH: &str = "22222222-2222-4222-8222-222222222222";
const INVOCATION_ID: &str = "33333333-3333-4333-8333-333333333333";
const HOOK_TOKEN: &str = "44444444-4444-4444-8444-444444444444";
const EVENT_ID: &str = "5aaaaaaa-5555-4555-8555-555555555555";
const AGENT_SESSION_ID: &str = "66666666-6666-4666-8666-666666666666";
const WORKER_PID: u32 = 101;
const SHELL_PID: u32 = 102;
const AGENT_PID: u32 = 103;
const HOOK_PID: u32 = 104;
const WORKER_START: u64 = 201;
const SHELL_START: u64 = 202;
const AGENT_START: u64 = 203;
const HOOK_START: u64 = 204;
const PROJECT_COMPONENT: &str = "-project";
const TEST_CWD: &str = "/tmp/yolo-fixture/project";
const NEXT_CWD: &str = "/tmp/yolo-fixture/project/next";

#[test]
fn claude_hook_wire_round_trips_all_supported_lifecycle_events() {
    let base = common(TEST_CWD);
    let events = vec![
        ClaudeHookInput::SessionStart {
            common: base.clone(),
            source: SessionStartSource::Startup,
        },
        ClaudeHookInput::SessionEnd {
            common: base.clone(),
            reason: SessionEndReason::Clear,
        },
        ClaudeHookInput::CwdChanged {
            common: common(NEXT_CWD),
            old_cwd: TEST_CWD.into(),
            new_cwd: NEXT_CWD.into(),
        },
        ClaudeHookInput::PreCompact {
            common: base.clone(),
            trigger: CompactTrigger::Manual,
            custom_instructions: String::new(),
        },
        ClaudeHookInput::PostCompact {
            common: base.clone(),
            trigger: CompactTrigger::Auto,
            compact_summary: "summary".into(),
        },
        ClaudeHookInput::UserPromptSubmit {
            common: base.clone(),
            prompt: "prompt".into(),
        },
        ClaudeHookInput::Stop {
            common: base,
            stop_hook_active: false,
            last_assistant_message: "done".into(),
        },
    ];
    for input in events {
        let frame = frame(input);
        frame.validate().unwrap();
        let encoded = serde_json::to_vec(&frame).unwrap();
        let decoded: ClaudeHookFrame = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, frame);
    }
}

#[test]
fn session_start_uses_the_real_claude_event_and_source_names() {
    let frame = frame(ClaudeHookInput::SessionStart {
        common: common(TEST_CWD),
        source: SessionStartSource::Startup,
    });
    let value = serde_json::to_value(frame).unwrap();
    assert_eq!(
        value[CLAUDE_INPUT_KEY][CLAUDE_EVENT_NAME_KEY],
        CLAUDE_SESSION_START_EVENT
    );
    assert_eq!(
        value[CLAUDE_INPUT_KEY][CLAUDE_SOURCE_KEY],
        CLAUDE_STARTUP_SOURCE
    );
}

#[test]
fn wire_validation_rejects_noncanonical_identity_path_and_lineage() {
    let mut uppercase = frame(start());
    uppercase.event_id = EVENT_ID.to_uppercase();
    assert_eq!(uppercase.validate(), Err(WIRE_EVENT_INVALID));

    let mut wrong_path = frame(start());
    if let ClaudeHookInput::SessionStart { common, .. } = &mut wrong_path.input {
        common.transcript_path = PathBuf::from("/tmp/not-the-session.jsonl");
    }
    assert_eq!(wrong_path.validate(), Err(WIRE_EVENT_INVALID));

    let mut broken_lineage = frame(start());
    broken_lineage.authority.hook.parent_pid = Some(SHELL_PID);
    assert_eq!(broken_lineage.validate(), Err(WIRE_AUTHORITY_INVALID));
}

#[test]
fn exact_authority_distinguishes_pid_reuse_token_and_runtime_epoch() {
    let expected = authority();
    let mut reused = expected.clone();
    reused.agent.start_time += 1;
    assert_ne!(reused, expected);
    let mut token = expected.clone();
    token.hook_token = EVENT_ID.into();
    assert_ne!(token, expected);
    let mut epoch = expected.clone();
    epoch.runtime_epoch = EVENT_ID.into();
    assert_ne!(epoch, expected);
}

#[test]
fn missing_required_wire_fields_are_rejected() {
    let mut value = serde_json::to_value(frame(start())).unwrap();
    value.as_object_mut().unwrap().remove(CLAUDE_EVENT_ID_KEY);
    assert!(serde_json::from_value::<ClaudeHookFrame>(value).is_err());
    assert!(serde_json::from_value::<ClaudeHookFrame>(Value::Null).is_err());
}

fn frame(input: ClaudeHookInput) -> ClaudeHookFrame {
    ClaudeHookFrame {
        schema_version: WIRE_SCHEMA_VERSION,
        event_id: EVENT_ID.into(),
        authority: authority(),
        input,
    }
}

fn authority() -> HookAuthority {
    HookAuthority {
        terminal_id: YOLO_SESSION_ID.into(),
        runtime_epoch: RUNTIME_EPOCH.into(),
        invocation_id: INVOCATION_ID.into(),
        hook_token: HOOK_TOKEN.into(),
        worker: identity(WORKER_PID, WORKER_START, None),
        shell: identity(SHELL_PID, SHELL_START, Some(WORKER_PID)),
        agent: identity(AGENT_PID, AGENT_START, Some(SHELL_PID)),
        hook: identity(HOOK_PID, HOOK_START, Some(AGENT_PID)),
    }
}

fn identity(pid: u32, start_time: u64, parent_pid: Option<u32>) -> ProcessIdentity {
    ProcessIdentity {
        pid,
        start_time,
        parent_pid,
    }
}

fn start() -> ClaudeHookInput {
    ClaudeHookInput::SessionStart {
        common: common(TEST_CWD),
        source: SessionStartSource::Startup,
    }
}

fn common(cwd: &str) -> CommonHookInput {
    CommonHookInput {
        session_id: AGENT_SESSION_ID.into(),
        transcript_path: transcript(
            Path::new("/tmp/yolo-fixture/claude"),
            PROJECT_COMPONENT,
            AGENT_SESSION_ID,
        ),
        cwd: cwd.into(),
    }
}
