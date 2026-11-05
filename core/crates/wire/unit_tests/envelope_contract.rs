use super::*;
use serde::de::DeserializeOwned;

const REQUEST_ID: &str = "current-envelope";

fn map<T: Serialize>(value: &T) -> Vec<(Value, Value)> {
    let Value::Map(fields) = to_value(value).unwrap() else {
        panic!("envelope must be a structural map");
    };
    fields
}

fn strict_payload<T: Serialize + DeserializeOwned>(message: &T) {
    let fields = map(message);
    assert!(
        fields
            .iter()
            .any(|(key, value)| { key.as_str() == Some(field::PAYLOAD) && *value == Value::Null })
    );
    let canonical = encode(message).unwrap();
    assert!(decode::<T>(&canonical[FRAME_LENGTH_BYTES..]).is_ok());

    let missing = fields
        .into_iter()
        .filter(|(key, _)| key.as_str() != Some(field::PAYLOAD))
        .collect();
    let incomplete = encode(&Value::Map(missing)).unwrap();
    assert!(decode::<T>(&incomplete[FRAME_LENGTH_BYTES..]).is_err());
}

fn rejects_extra<T: Serialize + DeserializeOwned>(message: &T) {
    let mut fields = map(message);
    fields.push((Value::from(field::ROWS), Value::Null));
    assert!(from_value::<T>(Value::Map(fields)).is_err());
}

#[test]
fn current_request_response_failure_and_event_require_explicit_payload() {
    let request = RequestEnvelope::new(REQUEST_ID, method::HOST_STATUS, Value::Null);
    strict_payload(&request);
    strict_payload(&ResponseEnvelope::empty(&request));
    strict_payload(&ResponseEnvelope::failure(
        &request,
        ErrorBody::internal("failed"),
    ));
    strict_payload(&EventEnvelope::new(event::SESSION_UPDATED, None, None, &()).unwrap());
}

#[test]
fn envelope_metadata_rejects_extensions_while_domain_payloads_remain_structural() {
    let request = RequestEnvelope::new(REQUEST_ID, method::HOST_STATUS, Value::Null);
    rejects_extra(&request);
    rejects_extra(&ResponseEnvelope::empty(&request));
    rejects_extra(&EventEnvelope::new(event::SESSION_UPDATED, None, None, &()).unwrap());
    rejects_extra(&ErrorBody::internal("failed"));

    let payload = Value::Map(vec![(Value::from(field::ROWS), Value::Null)]);
    let request = RequestEnvelope::new(REQUEST_ID, method::HELLO, payload.clone());
    assert_eq!(
        from_value::<RequestEnvelope>(to_value(&request).unwrap())
            .unwrap()
            .payload,
        payload
    );
}

#[test]
fn nested_error_metadata_does_not_silently_discard_unknown_fields() {
    let request = RequestEnvelope::new(REQUEST_ID, method::HOST_STATUS, Value::Null);
    let error = ErrorBody::internal("failed");
    let mut response = map(&ResponseEnvelope::failure(&request, error.clone()));
    let mut extended = map(&error);
    extended.push((Value::from(field::ROWS), Value::Null));
    *response
        .iter_mut()
        .find(|(key, _)| key.as_str() == Some(field::ERROR))
        .unwrap() = (Value::from(field::ERROR), Value::Map(extended));
    assert!(from_value::<ResponseEnvelope>(Value::Map(response)).is_err());
}
