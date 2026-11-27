use crate::GeminiHookCodec;
use crate::constants::*;
use yolo_hook_codec::HookCodec;
use yolo_hooks_runtime::EventKind;
use uuid::Uuid;

fn payload(event: &str) -> serde_json::Value {
    let transcript = "/home/dev/.gemini/tmp/p/chats/session-x.jsonl";
    let mut value = serde_json::json!({
        "cwd": "/home/dev",
        "source": "startup",
        "reason": "exit",
    });
    let object = value.as_object_mut().unwrap();
    object.insert(PAYLOAD_HOOK_EVENT_NAME.into(), event.into());
    object.insert(PAYLOAD_SESSION_ID.into(), Uuid::new_v4().to_string().into());
    object.insert(PAYLOAD_TRANSCRIPT_PATH.into(), transcript.into());
    value
}

#[test]
fn lifecycle_events_map_to_canonical_facts() {
    let codec = GeminiHookCodec;
    let start = codec.decode(&payload("SessionStart")).unwrap().unwrap();
    assert!(matches!(start.event, EventKind::SessionStart { .. }));
    let prompt = codec.decode(&payload("BeforeAgent")).unwrap().unwrap();
    assert_eq!(prompt.event, EventKind::UserPromptSubmit);
    let end = codec.decode(&payload("AfterAgent")).unwrap().unwrap();
    assert_eq!(end.event, EventKind::TurnEnded);
    let session_end = codec.decode(&payload("SessionEnd")).unwrap().unwrap();
    assert!(matches!(session_end.event, EventKind::SessionEnd { .. }));
}

#[test]
fn resume_start_resolves_to_the_durable_session_transcript() {
    use yolo_hooks_runtime::SessionStartSource;
    let root = tempfile::tempdir().unwrap();
    let chats = root.path().join("chats");
    std::fs::create_dir_all(&chats).unwrap();
    let session = Uuid::new_v4().to_string();
    let short = &session[..8];
    let old = chats.join(format!("session-2026-10-29T03-30-{short}.jsonl"));
    let new = chats.join(format!("session-2026-10-29T04-00-{short}.jsonl"));
    let header = format!("{{\"sessionId\":\"{session}\",\"projectHash\":\"hash\"}}\n");
    std::fs::write(&old, format!("{header}{}", "x".repeat(2048))).unwrap();
    std::fs::write(&new, header).unwrap();
    let mut resume = payload("SessionStart");
    resume[PAYLOAD_SESSION_ID] = session.into();
    resume["source"] = "resume".into();
    resume[PAYLOAD_TRANSCRIPT_PATH] = new.to_string_lossy().into_owned().into();
    let event = GeminiHookCodec.decode(&resume).unwrap().unwrap();
    assert_eq!(
        event.event,
        EventKind::SessionStart {
            source: SessionStartSource::Resume
        }
    );
    assert_eq!(event.transcript_path, old);
}

#[test]
fn telemetry_events_are_ignored_and_bad_sessions_rejected() {
    let codec = GeminiHookCodec;
    assert!(codec.decode(&payload("AfterModel")).unwrap().is_none());
    let mut bad = payload("SessionStart");
    bad[PAYLOAD_SESSION_ID] = "not-a-session".into();
    assert!(codec.decode(&bad).is_err());
}
