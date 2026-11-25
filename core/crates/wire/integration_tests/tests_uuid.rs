use super::{
    EventEnvelope, FRAME_LENGTH_BYTES, RequestEnvelope, ResponseEnvelope, decode, encode,
    from_value, to_value,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
use yolo_capnp_value::Value;
use yolo_terminal_model::{
    OutputChunk, RecoveryState, RuntimeView, TerminalManifest, TerminalRuntimeStatus, TerminalView,
};

const TEST_WORKER_PID: u32 = 10;
const TEST_CHILD_PID: u32 = 11;
const TEST_WORKER_START_TIME: u64 = TEST_WORKER_PID as u64;
const TEST_CHILD_START_TIME: u64 = TEST_CHILD_PID as u64;
const REMOVED_AGENT_MANIFEST_FIELDS: &[&str] = &[
    "provider",
    super::field::PROVIDER_SESSION_ID,
    "codex_home",
    "env_profile_refs",
];

fn session_view(seed: u128) -> TerminalView {
    TerminalView {
        manifest: TerminalManifest {
            schema_version: 1,
            terminal_id: Uuid::from_u128(seed),
            name: format!("session-{seed}"),
            command: "/bin/sh".into(),
            args: Vec::new(),
            cwd: PathBuf::from("/tmp"),
            cols: 120,
            rows: 32,
            created_at_ms: 1,
            updated_at_ms: 2,
            archived: false,
            recovery_state: RecoveryState::Created,
        },
        runtime: Some(RuntimeView {
            presentation: Default::default(),
            runtime_epoch: Uuid::from_u128(seed + 1),
            worker_id: Uuid::from_u128(seed + 2),
            worker_pid: TEST_WORKER_PID,
            worker_start_time: TEST_WORKER_START_TIME,
            child_pid: TEST_CHILD_PID,
            child_start_time: TEST_CHILD_START_TIME,
            status: TerminalRuntimeStatus::Running,
            last_output_sequence: 12,
            exit: None,
            bootstrap_delivery: yolo_terminal_model::BootstrapDelivery::NotRequired,
            capabilities: Some(Vec::new()),
            fault_reap: None,
        }),
        agent: None,
        completed_attachment: None,
    }
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value
        .as_map()
        .unwrap()
        .iter()
        .find(|(candidate, _)| candidate.as_str() == Some(key))
        .map(|(_, value)| value)
        .unwrap()
}

fn raw_wire_value<T: Serialize>(value: &T) -> Value {
    let frame = encode(value).unwrap();
    decode(&frame[FRAME_LENGTH_BYTES..]).unwrap()
}

fn assert_session_ids_are_strings(value: &Value) {
    let manifest = field(value, super::field::MANIFEST);
    let runtime = field(value, super::field::RUNTIME);
    assert!(field(manifest, super::field::TERMINAL_ID).as_str().is_some());
    assert!(
        field(runtime, super::field::RUNTIME_EPOCH)
            .as_str()
            .is_some()
    );
    assert!(field(runtime, super::field::WORKER_ID).as_str().is_some());
    assert_eq!(
        field(runtime, super::field::WORKER_START_TIME).as_u64(),
        Some(TEST_WORKER_START_TIME)
    );
    assert_eq!(
        field(runtime, super::field::CHILD_START_TIME).as_u64(),
        Some(TEST_CHILD_START_TIME)
    );
}

#[test]
fn create_and_list_responses_encode_uuid_fields_as_strings() {
    let request = RequestEnvelope::new("id", super::method::CREATE_SESSION, Value::Null);
    let create = ResponseEnvelope::success(&request, &session_view(1)).unwrap();
    assert_session_ids_are_strings(field(&raw_wire_value(&create), super::field::PAYLOAD));

    let list =
        ResponseEnvelope::success(&request, &vec![session_view(10), session_view(20)]).unwrap();
    let wire = raw_wire_value(&list);
    for session in field(&wire, super::field::PAYLOAD).as_list().unwrap() {
        assert_session_ids_are_strings(session);
    }
}

#[test]
fn removed_agent_manifest_fields_are_rejected() {
    for removed in REMOVED_AGENT_MANIFEST_FIELDS {
        let Value::Map(mut fields) = to_value(&session_view(1).manifest).unwrap() else {
            panic!("manifest must encode as a map");
        };
        fields.push((Value::from(*removed), Value::Null));
        assert!(from_value::<TerminalManifest>(Value::Map(fields)).is_err());
    }
}

#[test]
fn event_payload_encodes_uuid_as_string_and_bytes_as_binary() {
    let chunk = OutputChunk {
        runtime_epoch: Uuid::from_u128(30),
        sequence: 2,
        data: vec![0, 159, 255],
    };
    let event = EventEnvelope::new(super::event::TERMINAL_OUTPUT, None, None, &chunk).unwrap();
    let wire = raw_wire_value(&event);
    let payload = field(&wire, super::field::PAYLOAD);
    assert!(
        field(payload, super::field::RUNTIME_EPOCH)
            .as_str()
            .is_some()
    );
    assert!(matches!(field(payload, super::field::DATA), Value::Data(_)));
}

#[derive(Debug, Serialize, Deserialize)]
struct FutureWireDto {
    id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
struct StrictWireDto {
    #[serde(with = "yolo_crypto::serde_uuid")]
    id: Uuid,
}

#[test]
fn global_codec_uses_uuid_strings_and_strict_fields_reject_noncanonical_input() {
    let id = Uuid::parse_str("abcdefab-cdef-abcd-efab-cdefabcdefab").unwrap();
    assert!(
        field(&to_value(&FutureWireDto { id }).unwrap(), "id")
            .as_str()
            .is_some()
    );

    for invalid in [
        Value::List(id.as_bytes().iter().copied().map(Value::from).collect()),
        Value::Data(id.as_bytes().to_vec()),
        Value::from(id.hyphenated().to_string().to_uppercase()),
    ] {
        let value = Value::Map(vec![(Value::from("id"), invalid)]);
        assert!(from_value::<StrictWireDto>(value).is_err());
    }
}
