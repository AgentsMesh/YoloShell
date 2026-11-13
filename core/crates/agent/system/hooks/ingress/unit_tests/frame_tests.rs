use super::fixture::*;
use crate::{HookProcessIdentity, HookScope};
use uuid::Uuid;

#[test]
fn outbox_orders_sequences_per_scope() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::set_permissions(
        directory.path(),
        std::os::unix::fs::PermissionsExt::from_mode(0o700),
    )
    .unwrap();
    let path = directory.path().join("native-state.json");
    let first = hook_scope(HookProcessIdentity {
        pid: 30,
        start_time: 30,
    });
    let second = hook_scope(HookProcessIdentity {
        pid: 31,
        start_time: 31,
    });
    let enqueue = |scope, sequence| {
        crate::outbox::enqueue(
            &path,
            scope,
            "test",
            Uuid::from_u128(sequence as u128),
            serde_json::json!({"sequence": sequence}),
        )
        .unwrap()
    };
    assert_eq!(enqueue(first, 1), 1);
    assert_eq!(enqueue(first, 2), 2);
    assert_eq!(enqueue(second, 3), 1);
    let request = crate::outbox::next(&path, first).unwrap().unwrap();
    assert_eq!(request.event_sequence, 1);
    crate::outbox::acknowledge(&path, first, 1).unwrap();
    let request = crate::outbox::next(&path, first).unwrap().unwrap();
    assert_eq!(request.event_sequence, 2);
    assert!(
        crate::outbox::acknowledge(&path, first, 3).is_err(),
        "cannot acknowledge a later sequence over a pending prefix"
    );
    crate::outbox::acknowledge(&path, first, 2).unwrap();
    assert!(crate::outbox::next(&path, first).unwrap().is_none());
    let other = crate::outbox::next(&path, second).unwrap().unwrap();
    assert_eq!(other.event_sequence, 1);
}

#[test]
fn request_and_response_frames_round_trip() {
    let request = crate::contract::HookRequest {
        schema_version: crate::contract::SCHEMA_VERSION,
        provider: "test".into(),
        scope: hook_scope(HookProcessIdentity {
            pid: 30,
            start_time: 30,
        }),
        event_sequence: 1,
        event_id: Uuid::from_u128(7),
        payload: serde_json::json!({(yolo_gemini_hooks::PAYLOAD_HOOK_EVENT_NAME): "SessionStart"}),
    };
    assert!(request.is_valid());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let (mut writer, mut reader) = tokio::io::duplex(4096);
        crate::contract::write_frame(&mut writer, &request)
            .await
            .unwrap();
        let decoded: crate::contract::HookRequest =
            crate::contract::read_frame(&mut reader).await.unwrap();
        assert_eq!(decoded, request);
        let response = crate::contract::HookResponse::rejection(&request);
        assert!(!response.acknowledges(&request));
    });
}
