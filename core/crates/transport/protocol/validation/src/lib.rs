use uuid::Uuid;
use yolo_capnp_value::Value;
use yolo_wire::{
    EVENT_KIND, EventEnvelope, PROTOCOL_VERSION, ProtocolError, RESPONSE_KIND, RequestEnvelope,
    ResponseEnvelope, event, field,
};

pub fn validate_response(
    request: &RequestEnvelope,
    response: &ResponseEnvelope,
) -> Result<(), ProtocolError> {
    if response.version != PROTOCOL_VERSION
        || response.kind != RESPONSE_KIND
        || response.request_id != request.request_id
        || response.method != request.method
        || response.ok && response.error.is_some()
        || !response.ok && (response.error.is_none() || response.payload != Value::Null)
    {
        return Err(ProtocolError::ResponseMismatch);
    }
    Ok(())
}

pub fn validate_event(envelope: &EventEnvelope) -> Result<(), ProtocolError> {
    if envelope.version != PROTOCOL_VERSION || envelope.kind != EVENT_KIND {
        return Err(ProtocolError::EventMismatch);
    }
    Ok(())
}

pub fn validate_attachment_event(
    envelope: &EventEnvelope,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
) -> Result<(), ProtocolError> {
    validate_event(envelope)?;
    if runtime_epoch.is_nil() {
        return Err(ProtocolError::EventMismatch);
    }
    let event_session = envelope.terminal_id.as_deref().and_then(parse_uuid);
    let event_epoch = envelope.runtime_epoch.as_deref().and_then(parse_uuid);
    if event_session != Some(terminal_id) || event_epoch != Some(runtime_epoch) {
        return Err(ProtocolError::EventMismatch);
    }
    if envelope.event == event::TERMINAL_OUTPUT
        && output_runtime_epoch(&envelope.payload) != Some(runtime_epoch)
    {
        return Err(ProtocolError::EventMismatch);
    }
    if envelope.event == event::SESSION_EXITED
        && exit_runtime_epoch(&envelope.payload) != Some(runtime_epoch)
    {
        return Err(ProtocolError::EventMismatch);
    }
    Ok(())
}

fn parse_uuid(value: &str) -> Option<Uuid> {
    yolo_crypto::serde_uuid::parse_canonical(value)
}

fn output_runtime_epoch(payload: &Value) -> Option<Uuid> {
    let Value::Map(entries) = payload else {
        return None;
    };
    let mut epochs = entries
        .iter()
        .filter(|(key, _)| key.as_str() == Some(field::RUNTIME_EPOCH));
    let epoch = epochs.next()?.1.as_str().and_then(parse_uuid);
    (epochs.next().is_none()).then_some(epoch).flatten()
}

fn exit_runtime_epoch(payload: &Value) -> Option<Uuid> {
    yolo_wire::from_value::<yolo_terminal_model::ExitRecord>(payload.clone())
        .ok()
        .map(|exit| exit.runtime_epoch)
}
