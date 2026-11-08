use crate::{runtime_request, runtime_run_request};
use uuid::Uuid;
use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, TERMINAL_PROTOCOL_SCHEMA_VERSION, TerminalRunFields,
    TerminalRunRequest, TerminalSize,
};

fn fields() -> CreateShellFields {
    CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        creation_token: Uuid::new_v4(),
        name: Some("original-query-name".into()),
        cwd: "~/original-query-directory".into(),
        size: TerminalSize::new(91, 26).unwrap(),
    }
}

#[test]
fn original_public_request_id_is_not_part_of_host_idempotency_identity() {
    let original = CreateShellRequest::new(fields()).unwrap();
    let mut changed = original.fields().clone();
    changed.request_id = Uuid::new_v4();
    let next = CreateShellRequest::new(changed).unwrap();
    assert_ne!(original.fields().request_id, next.fields().request_id);
    assert_eq!(runtime_request(&original), runtime_request(&next));
    assert_eq!(
        serde_json::to_vec(&runtime_request(&original)).unwrap(),
        serde_json::to_vec(&runtime_request(&next)).unwrap()
    );
}

#[test]
fn run_identity_carries_the_bounded_input_and_drops_the_public_request_id() {
    let original = TerminalRunRequest::new(run_fields()).unwrap();
    let mut changed = original.fields().clone();
    changed.request_id = Uuid::new_v4();
    let next = TerminalRunRequest::new(changed).unwrap();
    let runtime = runtime_run_request(&original, None);
    assert_eq!(runtime.creation_token, original.fields().creation_token);
    assert_eq!(runtime.name.as_deref(), Some("original-query-name"));
    assert_eq!(runtime.cols, Some(91));
    assert_eq!(runtime.rows, Some(26));
    assert_eq!(
        runtime.initial_input.as_deref(),
        Some(b"codex\r".as_slice())
    );
    assert_eq!(
        serde_json::to_vec(&runtime).unwrap(),
        serde_json::to_vec(&runtime_run_request(&next, None)).unwrap()
    );
}

fn run_fields() -> TerminalRunFields {
    TerminalRunFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        creation_token: Uuid::new_v4(),
        name: Some("original-query-name".into()),
        cwd: "~/original-query-directory".into(),
        size: TerminalSize::new(91, 26).unwrap(),
        input: b"codex\r".to_vec(),
        workspace_id: None,
    }
}
