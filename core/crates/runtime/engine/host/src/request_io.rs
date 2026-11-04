use super::*;

pub(crate) fn respond<T: Serialize>(
    request: &RequestEnvelope,
    payload: &T,
) -> std::result::Result<ResponseEnvelope, ErrorBody> {
    ResponseEnvelope::success(request, payload).map_err(internal)
}

pub(crate) fn decode_request<T: serde::de::DeserializeOwned>(
    request: &RequestEnvelope,
) -> std::result::Result<T, ErrorBody> {
    request
        .decode_payload()
        .map_err(|error| ErrorBody::invalid(error.to_string()))
}

pub(crate) fn internal(error: impl std::fmt::Display) -> ErrorBody {
    ErrorBody::internal(error.to_string())
}

pub(crate) fn extract_runtime_target(
    value: &yolo_capnp_value::Value,
) -> std::result::Result<(Uuid, Uuid), ErrorBody> {
    let yolo_capnp_value::Value::Map(fields) = value else {
        return Err(ErrorBody::invalid(REQUEST_RUNTIME_SESSION_MISSING));
    };
    let terminal_id = extract_runtime_uuid(
        fields,
        crate::protocol::field::TERMINAL_ID,
        REQUEST_RUNTIME_SESSION_MISSING,
        REQUEST_RUNTIME_SESSION_INVALID,
    )?;
    let runtime_epoch = extract_runtime_uuid(
        fields,
        crate::protocol::field::RUNTIME_EPOCH,
        REQUEST_RUNTIME_EPOCH_MISSING,
        REQUEST_RUNTIME_EPOCH_INVALID,
    )?;
    Ok((terminal_id, runtime_epoch))
}

pub(crate) fn extract_runtime_uuid(
    fields: &[(yolo_capnp_value::Value, yolo_capnp_value::Value)],
    field_name: &str,
    missing: &'static str,
    invalid: &'static str,
) -> std::result::Result<Uuid, ErrorBody> {
    let mut values = fields
        .iter()
        .filter(|(key, _)| key.as_str() == Some(field_name));
    let value = values.next().ok_or_else(|| ErrorBody::invalid(missing))?;
    if values.next().is_some() {
        return Err(ErrorBody::invalid(invalid));
    }
    value
        .1
        .as_str()
        .and_then(crate::serde_uuid::parse_canonical)
        .filter(|value| !value.is_nil())
        .ok_or_else(|| ErrorBody::invalid(invalid))
}
