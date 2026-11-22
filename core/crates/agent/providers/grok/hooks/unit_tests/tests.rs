use crate::GrokHookCodec;
use crate::constants::*;
use crate::transcript::session_transcript;
use std::path::PathBuf;
use yolo_agent_contract::{AgentKind, ObservedProcess};
use yolo_hook_codec::HookCodec;
use yolo_hooks_runtime::{EventKind, SessionStartSource};
use uuid::Uuid;

fn payload(event: &str) -> serde_json::Value {
    let transcript = "/home/dev/.grok/sessions/p/s/updates.jsonl";
    let mut value = serde_json::json!({
        "hookEventName": event,
        "sessionId": Uuid::new_v4().to_string(),
        "transcriptPath": transcript,
        "cwd": "/home/dev",
        "source": "new",
        "reason": "shutdown",
        "promptId": "prompt",
        "stopHookActive": false,
    });
    let object = value.as_object_mut().unwrap();
    object.insert(PAYLOAD_HOOK_EVENT_NAME.into(), pascal(event).into());
    object.insert(PAYLOAD_SESSION_ID.into(), Uuid::new_v4().to_string().into());
    object.insert(PAYLOAD_TRANSCRIPT_PATH.into(), transcript.into());
    value
}

fn pascal(event: &str) -> String {
    event
        .split('_')
        .map(|part| {
            let mut letters = part.chars();
            letters
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + letters.as_str())
                .unwrap_or_default()
        })
        .collect()
}

#[test]
fn lifecycle_events_map_to_canonical_facts() {
    let codec = GrokHookCodec;
    let start = codec.decode(&payload(EVENT_SESSION_START_SNAKE)).unwrap().unwrap();
    assert_eq!(
        start.event,
        EventKind::SessionStart {
            source: SessionStartSource::Startup
        }
    );
    let prompt = codec.decode(&payload(EVENT_USER_PROMPT_SUBMIT_SNAKE)).unwrap().unwrap();
    assert_eq!(prompt.event, EventKind::UserPromptSubmit);
    let mut completion = payload("stop");
    completion["reason"] = REASON_END_TURN.into();
    let stop = codec.decode(&completion).unwrap().unwrap();
    assert_eq!(stop.event, EventKind::TurnEnded);
    // A shutdown stop follows SessionEnd and is not a turn fact.
    assert!(codec.decode(&payload("stop")).unwrap().is_none());
    let cancelled = codec.decode(&payload(EVENT_STOP_CANCELLED_SNAKE)).unwrap().unwrap();
    assert_eq!(cancelled.event, EventKind::TurnEnded);
    let failure = codec.decode(&payload(EVENT_STOP_FAILURE_SNAKE)).unwrap().unwrap();
    assert_eq!(failure.event, EventKind::StopFailure);
    let end = codec.decode(&payload(EVENT_SESSION_END_SNAKE)).unwrap().unwrap();
    assert!(matches!(end.event, EventKind::SessionEnd { .. }));
}

#[test]
fn a_loaded_session_is_a_resume_start() {
    let mut value = payload(EVENT_SESSION_START_SNAKE);
    value["source"] = "load".into();
    assert_eq!(
        GrokHookCodec.decode(&value).unwrap().unwrap().event,
        EventKind::SessionStart {
            source: SessionStartSource::Resume
        }
    );
    let mut value = payload(EVENT_SESSION_START_SNAKE);
    value["source"] = "new".into();
    assert_eq!(
        GrokHookCodec.decode(&value).unwrap().unwrap().event,
        EventKind::SessionStart {
            source: SessionStartSource::Startup
        }
    );
}

#[test]
fn pascal_alias_alone_also_decodes() {
    let mut value = payload("stop");
    value["reason"] = REASON_END_TURN.into();
    value.as_object_mut().unwrap().remove("hookEventName");
    assert_eq!(
        GrokHookCodec.decode(&value).unwrap().unwrap().event,
        EventKind::TurnEnded
    );
}

#[test]
fn a_start_without_a_transcript_derives_the_session_path() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(".grok");
    let cwd = root.path().join("project/grok");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&cwd).unwrap();
    let home = home.canonicalize().unwrap();
    let cwd = cwd.canonicalize().unwrap();
    let session = Uuid::new_v4().to_string();
    let mut value = payload(EVENT_SESSION_START_SNAKE);
    value["sessionId"] = session.clone().into();
    value[PAYLOAD_SESSION_ID] = session.clone().into();
    value["cwd"] = cwd.to_string_lossy().into_owned().into();
    value.as_object_mut().unwrap().remove("transcriptPath");
    value.as_object_mut().unwrap().remove(PAYLOAD_TRANSCRIPT_PATH);
    let event = GrokHookCodec.decode(&value).unwrap().unwrap();
    assert_eq!(event.transcript_path, PathBuf::new());
    let agent = ObservedProcess {
        pid: 1,
        start_time: 1,
        parent_pid: 0,
        process_group: 1,
        foreground_group: 1,
        executable: "grok".into(),
        argv: vec!["grok".into()],
        environment: Some(
            [("GROK_HOME".to_owned(), home.to_string_lossy().into_owned())]
                .into_iter()
                .collect(),
        ),
        cwd: Some(cwd.clone()),
        evidence_paths: None,
    };
    assert_eq!(GrokHookCodec.kind(), AgentKind::Other("grok".into()));
    let resolved = GrokHookCodec
        .resolve_transcript_path(&agent, &event)
        .unwrap();
    assert_eq!(resolved, session_transcript(&home, &cwd, &session));
}

#[test]
fn telemetry_events_are_ignored_and_bad_sessions_rejected() {
    let codec = GrokHookCodec;
    assert!(codec.decode(&payload(EVENT_TASK_COMPLETE_SNAKE)).unwrap().is_none());
    let mut bad = payload(EVENT_SESSION_START_SNAKE);
    bad["sessionId"] = "not-a-session".into();
    assert!(codec.decode(&bad).is_err());
}

#[test]
fn session_transcripts_percent_encode_the_working_directory() {
    assert_eq!(
        session_transcript(
            std::path::Path::new("/home/dev/.grok"),
            std::path::Path::new("/workspace/my app"),
            "01a186cc-b1a5-7842-a9dc-784a4b320c5d"
        ),
        PathBuf::from(
            "/home/dev/.grok/sessions/%2Fworkspace%2Fmy%20app/01a186cc-b1a5-7842-a9dc-784a4b320c5d/updates.jsonl"
        )
    );
}
