use super::LeaseRequest;
use super::constants::code;
use yolo_terminal_model::WriterLease;
use yolo_wire::ErrorBody;

pub(super) fn lease_error(
    code: &'static str,
    message: &'static str,
    current: Option<&WriterLease>,
    request: Option<&LeaseRequest>,
) -> ErrorBody {
    ErrorBody::lease(
        code,
        message,
        serde_json::json!({
            (yolo_wire::field::HOLDER_ID): current.map(|lease| lease.holder_id.as_str()),
            (yolo_wire::field::CONTROLLER_INSTANCE_ID): current
                .and_then(|lease| lease.controller_instance_id.as_deref()),
            (yolo_wire::field::GENERATION): current.map(|lease| lease.generation),
            (yolo_wire::field::EXPIRES_AT_MS): current.map(|lease| lease.expires_at_ms),
            (yolo_wire::field::REQUESTED_HOLDER_ID): request
                .map(|request| request.holder_id.as_str()),
            (yolo_wire::field::REQUESTED_CONTROLLER_INSTANCE_ID): request
                .and_then(|request| request.controller_instance_id.as_deref()),
            (yolo_wire::field::REQUESTED_GENERATION): request
                .and_then(|request| request.generation),
        }),
    )
}

pub(super) fn validate_request(
    current: &WriterLease,
    request: &LeaseRequest,
) -> Result<(), ErrorBody> {
    if current.holder_id != request.holder_id {
        return Err(lease_error(
            code::HELD_BY_OTHER,
            "writer lease is held by another controller",
            Some(current),
            Some(request),
        ));
    }
    if instance_changed(
        &current.controller_instance_id,
        &request.controller_instance_id,
    ) {
        return Err(lease_error(
            code::CONTROLLER_INSTANCE_MISMATCH,
            "writer lease belongs to another controller incarnation",
            Some(current),
            Some(request),
        ));
    }
    if request.generation != Some(current.generation) {
        return Err(lease_error(
            code::STALE_GENERATION,
            "the writer lease generation is stale",
            Some(current),
            Some(request),
        ));
    }
    Ok(())
}

pub(super) fn instance_changed(current: &Option<String>, requested: &Option<String>) -> bool {
    current != requested
}
