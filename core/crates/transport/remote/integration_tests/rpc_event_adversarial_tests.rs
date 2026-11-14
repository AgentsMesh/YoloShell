use super::super::RemoteError;
use super::super::rpc::RemoteRpc;
use crate::model::{OutputChunk, PROTOCOL_VERSION};
use crate::protocol::{
    EventEnvelope, ProtocolError, ResponseEnvelope, event, field, method, to_value,
};

const INCOMPATIBLE_PROTOCOL_VERSION_OFFSET: u16 = 1;
const INVALID_EVENT_KIND: &str = "invalid-event-kind";
const MALFORMED_RUNTIME_EPOCH: &str = "malformed-runtime-epoch";
const EVENT_SEQUENCE: u64 = 1;

#[derive(Clone, Copy)]
enum EventCorruption {
    Version,
    Kind,
    Session,
    OldRuntime,
    MissingRuntime,
    NilRuntime,
    MalformedRuntime,
    NonCanonicalRuntime,
    ChunkRuntime,
    DuplicateChunkRuntime,
}

#[tokio::test]
async fn attachment_epoch_remote_rejects_stale_or_malformed_event_and_poisons() {
    let expected_session = uuid::Uuid::new_v4();
    let expected_epoch = uuid::Uuid::new_v4();
    for corruption in [
        EventCorruption::Version,
        EventCorruption::Kind,
        EventCorruption::Session,
        EventCorruption::OldRuntime,
        EventCorruption::MissingRuntime,
        EventCorruption::NilRuntime,
        EventCorruption::MalformedRuntime,
        EventCorruption::NonCanonicalRuntime,
        EventCorruption::ChunkRuntime,
        EventCorruption::DuplicateChunkRuntime,
    ] {
        let envelope = corrupted_event(expected_session, expected_epoch, corruption);
        let (client, server) = tokio::io::duplex(super::support::TEST_CHANNEL_CAPACITY);
        let server = super::rpc_fixture::serve(server, move |request| {
            let event = envelope.clone();
            Box::pin(async move { super::rpc_fixture::event_reply(ResponseEnvelope::empty(&request), event) })
        });
        let mut rpc = RemoteRpc::new(Box::new(client));
        rpc.request_until::<_, ()>(method::ATTACH_SESSION, &(), super::super::RemoteDeadline::operation()).await.unwrap();
        let error = rpc
            .next_event(expected_session, expected_epoch)
            .await
            .unwrap_err();
        if matches!(corruption, EventCorruption::DuplicateChunkRuntime) {
            assert!(matches!(error, RemoteError::Protocol(ProtocolError::Decode(_))));
        } else { assert!(matches!(error, RemoteError::Protocol(ProtocolError::EventMismatch))); }
        let error = rpc
            .next_event(expected_session, expected_epoch)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            RemoteError::Protocol(ProtocolError::ConnectionPoisoned)
        ));
        server.await.unwrap();
    }
}

fn corrupted_event(
    terminal_id: uuid::Uuid,
    runtime_epoch: uuid::Uuid,
    corruption: EventCorruption,
) -> EventEnvelope {
    let old_epoch = uuid::Uuid::new_v4();
    let chunk_epoch = match corruption {
        EventCorruption::OldRuntime | EventCorruption::ChunkRuntime => old_epoch,
        _ => runtime_epoch,
    };
    let mut envelope = EventEnvelope::new(
        event::TERMINAL_OUTPUT,
        Some(terminal_id.to_string()),
        Some(runtime_epoch.to_string()),
        &OutputChunk {
            runtime_epoch: chunk_epoch,
            sequence: EVENT_SEQUENCE,
            data: Vec::new(),
        },
    )
    .unwrap();
    match corruption {
        EventCorruption::Version => {
            envelope.version =
                PROTOCOL_VERSION.saturating_add(INCOMPATIBLE_PROTOCOL_VERSION_OFFSET);
        }
        EventCorruption::Kind => envelope.kind = INVALID_EVENT_KIND.into(),
        EventCorruption::Session => {
            envelope.terminal_id = Some(uuid::Uuid::new_v4().to_string());
        }
        EventCorruption::OldRuntime => envelope.runtime_epoch = Some(old_epoch.to_string()),
        EventCorruption::MissingRuntime => envelope.runtime_epoch = None,
        EventCorruption::NilRuntime => {
            envelope.runtime_epoch = Some(uuid::Uuid::nil().to_string());
        }
        EventCorruption::MalformedRuntime => {
            envelope.runtime_epoch = Some(MALFORMED_RUNTIME_EPOCH.into());
        }
        EventCorruption::NonCanonicalRuntime => {
            envelope.runtime_epoch = Some(runtime_epoch.simple().to_string());
        }
        EventCorruption::ChunkRuntime => {}
        EventCorruption::DuplicateChunkRuntime => {
            let yolo_capnp_value::Value::Map(mut fields) = envelope.payload else {
                panic!("output chunk must encode as a map");
            };
            fields.push((
                yolo_capnp_value::Value::from(field::RUNTIME_EPOCH),
                to_value(&runtime_epoch).unwrap(),
            ));
            envelope.payload = yolo_capnp_value::Value::Map(fields);
        }
    }
    envelope
}
