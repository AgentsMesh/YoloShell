use super::*;
use crate::model::{ExitReason, ExitRecord};
use crate::protocol::{ProtocolError, event, field};
use yolo_capnp_value::Value;

const TEST_SESSION_ID_VALUE: u128 = 1;
const EXPECTED_RUNTIME_EPOCH_VALUE: u128 = 2;
const WRONG_RUNTIME_EPOCH_VALUE: u128 = 3;
const EXIT_CODE: i32 = 0;
const EXIT_OBSERVED_AT_MS: i64 = 1;
const EXIT_LAST_OUTPUT_SEQUENCE: u64 = 0;
const ATTACH_NEXT_SEQUENCE: u64 = 1;
const MALFORMED_RUNTIME_EPOCH: &str = "malformed-runtime-epoch";

#[derive(Clone, Copy)]
enum ExitPayloadCorruption {
    MissingRuntimeEpoch,
    MalformedRuntimeEpoch,
    DuplicateRuntimeEpoch,
    WrongRuntimeEpoch,
}

#[tokio::test]
async fn session_exited_payload_epoch_rejection_poisons_attachment() {
    for corruption in [
        ExitPayloadCorruption::MissingRuntimeEpoch,
        ExitPayloadCorruption::MalformedRuntimeEpoch,
        ExitPayloadCorruption::DuplicateRuntimeEpoch,
        ExitPayloadCorruption::WrongRuntimeEpoch,
    ] {
        assert_exit_payload_epoch_rejection(corruption).await;
    }
}

async fn assert_exit_payload_epoch_rejection(corruption: ExitPayloadCorruption) {
    let terminal_id = Uuid::from_u128(TEST_SESSION_ID_VALUE);
    let runtime_epoch = Uuid::from_u128(EXPECTED_RUNTIME_EPOCH_VALUE);
    let (stream, peer) = crate::attachment_test_support::connected().await;
    let event = corrupted_exit_event(terminal_id, runtime_epoch, corruption);
    if matches!(corruption, ExitPayloadCorruption::DuplicateRuntimeEpoch) {
        let mut envelope = event.clone();
        envelope.payload = Value::Null;
        let Value::Map(mut raw) = yolo_capnp_value::to_value(&envelope).unwrap() else {
            unreachable!()
        };
        let (_, payload) = raw
            .iter_mut()
            .find(|(key, _)| key.as_str() == Some(field::PAYLOAD))
            .unwrap();
        *payload = event.payload;
        peer.raw_event(Value::Map(raw)).await.unwrap();
    } else {
        peer.event(&event).await.unwrap();
    }
    let mut attachment = attachment_fixture(stream, terminal_id, runtime_epoch);

    let error = attachment.next_event().await.unwrap_err();
    if matches!(corruption, ExitPayloadCorruption::DuplicateRuntimeEpoch) {
        assert!(matches!(
            error,
            ClientError::Protocol(
                ProtocolError::Io(_) | ProtocolError::Decode(_) | ProtocolError::EventMismatch
            )
        ));
    } else {
        assert!(matches!(
            error,
            ClientError::Protocol(ProtocolError::EventMismatch)
        ));
    }
    assert!(matches!(
        attachment.next_event().await,
        Err(ClientError::Protocol(ProtocolError::ConnectionPoisoned))
    ));
}

fn corrupted_exit_event(
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    corruption: ExitPayloadCorruption,
) -> EventEnvelope {
    let exit = ExitRecord {
        runtime_epoch,
        code: Some(EXIT_CODE),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: EXIT_OBSERVED_AT_MS,
        last_output_sequence: EXIT_LAST_OUTPUT_SEQUENCE,
    };
    let mut envelope = EventEnvelope::new(
        event::SESSION_EXITED,
        Some(terminal_id.to_string()),
        Some(runtime_epoch.to_string()),
        &exit,
    )
    .unwrap();
    let Value::Map(mut fields) = envelope.payload else {
        unreachable!();
    };
    match corruption {
        ExitPayloadCorruption::MissingRuntimeEpoch => {
            fields.retain(|(key, _)| key.as_str() != Some(field::RUNTIME_EPOCH));
        }
        ExitPayloadCorruption::MalformedRuntimeEpoch => {
            replace_runtime_epoch(&mut fields, Value::from(MALFORMED_RUNTIME_EPOCH));
        }
        ExitPayloadCorruption::DuplicateRuntimeEpoch => fields.push((
            Value::from(field::RUNTIME_EPOCH),
            Value::from(runtime_epoch.to_string()),
        )),
        ExitPayloadCorruption::WrongRuntimeEpoch => replace_runtime_epoch(
            &mut fields,
            Value::from(Uuid::from_u128(WRONG_RUNTIME_EPOCH_VALUE).to_string()),
        ),
    }
    envelope.payload = Value::Map(fields);
    envelope
}

fn replace_runtime_epoch(fields: &mut [(Value, Value)], replacement: Value) {
    let (_, epoch) = fields
        .iter_mut()
        .find(|(key, _)| key.as_str() == Some(field::RUNTIME_EPOCH))
        .unwrap();
    *epoch = replacement;
}

fn attachment_fixture(
    stream: yolo_runtime_rpc::RuntimeConnection,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
) -> Attachment {
    Attachment {
        stream: Some(stream),
        terminal_id,
        runtime_epoch,
        initial: AttachResult {
            runtime_epoch,
            snapshot: None,
            chunks: Vec::new(),
            next_sequence: ATTACH_NEXT_SEQUENCE,
            resynchronized: false,
        },
    }
}
