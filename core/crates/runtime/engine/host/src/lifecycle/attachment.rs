impl Host {
    pub(crate) async fn validate_published_attachment(
        &self,
        selected: &crate::model::RuntimeRecord,
    ) -> std::result::Result<(), ErrorBody> {
        let current = self.live_runtime_record(selected.terminal_id).await?;
        if attachment_runtime_matches(&current, selected)
            && current.controller_generation == self.controller_generation
        {
            return Ok(());
        }
        Err(ErrorBody::new(
            crate::protocol::error_code::IDENTITY_MISMATCH,
            WORKER_ATTACHMENT_RUNTIME_MISMATCH,
            false,
        ))
    }
}


pub(crate) fn validate_worker_attachment(
    request: &RequestEnvelope,
    response: &ResponseEnvelope,
    record: &crate::model::RuntimeRecord,
) -> std::result::Result<(), ErrorBody> {
    crate::protocol::validate_response(request, response)
        .map_err(|_| ErrorBody::protocol_incompatible(WORKER_ATTACHMENT_PROTOCOL_INVALID))?;
    if attachment_response_epoch(&response.payload)
        .is_some_and(|epoch| epoch != record.runtime_epoch)
    {
        return Err(ErrorBody::new(
            crate::protocol::error_code::IDENTITY_MISMATCH,
            WORKER_ATTACHMENT_RUNTIME_MISMATCH,
            false,
        ));
    }
    let attached: crate::worker::AttachResult = response
        .decode_payload()
        .map_err(|_| ErrorBody::protocol_incompatible(WORKER_ATTACHMENT_PROTOCOL_INVALID))?;
    let attach: AttachRequest = request
        .decode_payload()
        .map_err(|_| ErrorBody::protocol_incompatible(WORKER_ATTACHMENT_PROTOCOL_INVALID))?;
    attached
        .validate_runtime_epoch(record.runtime_epoch)
        .map_err(|_| {
            ErrorBody::new(
                crate::protocol::error_code::IDENTITY_MISMATCH,
                WORKER_ATTACHMENT_RUNTIME_MISMATCH,
                false,
            )
        })?;
    attached
        .validate_for_request(record.runtime_epoch, attach.after_seq)
        .map(|_| ())
        .map_err(|_| ErrorBody::protocol_incompatible(WORKER_ATTACHMENT_PROTOCOL_INVALID))
}

pub(crate) fn attachment_response_epoch(payload: &yolo_capnp_value::Value) -> Option<Uuid> {
    let yolo_capnp_value::Value::Map(fields) = payload else {
        return None;
    };
    let mut epochs = fields
        .iter()
        .filter(|(key, _)| key.as_str() == Some(crate::protocol::field::RUNTIME_EPOCH));
    let epoch = epochs.next()?.1.as_str()?;
    if epochs.next().is_some() {
        return None;
    }
    crate::serde_uuid::parse_canonical(epoch)
}

pub(crate) fn attachment_runtime_matches(
    current: &crate::model::RuntimeRecord,
    selected: &crate::model::RuntimeRecord,
) -> bool {
    current.has_same_process_identity(selected)
}
