use super::*;

pub(super) async fn handle_connection(mut stream: UnixStream, state: Arc<WorkerState>) -> Result<()> {
    local_transport::verify_same_uid(&stream)?;
    let auth_request: RequestEnvelope =
        tokio::time::timeout(WORKER_AUTH_TIMEOUT, read_worker_frame(&mut stream))
            .await
            .context(WORKER_AUTH_TIMEOUT_CONTEXT)??;
    let Some(opening_publication) =
        protocol_admission::admit(&mut stream, &state, &auth_request).await?
    else {
        return Ok(());
    };
    if auth_request.method == crate::pty_custody::METHOD {
        return secondary_custody_adoption::adopt_runtime(
            stream,
            auth_request,
            state,
            opening_publication,
        )
        .await;
    }
    if auth_request.method != WORKER_AUTH_METHOD {
        let response = ResponseEnvelope::failure(
            &auth_request,
            ErrorBody::new(
                crate::protocol::error_code::UNAUTHORIZED,
                "worker authentication required",
                false,
            ),
        );
        response_drain::write(&mut stream, &state, opening_publication, &response).await?;
        return Ok(());
    }
    let auth: WorkerAuth = auth_request.decode_payload()?;
    if auth.token != lock_recover(&state.record).auth_token {
        let response = ResponseEnvelope::failure(
            &auth_request,
            ErrorBody::new(
                crate::protocol::error_code::UNAUTHORIZED,
                "invalid worker token",
                false,
            ),
        );
        response_drain::write(&mut stream, &state, opening_publication, &response).await?;
        return Ok(());
    }
    let controller = match state
        .authenticate_ready_controller(auth.controller_generation)
        .await
    {
        Ok(controller) => controller,
        Err(error) => {
            response_drain::write(
                &mut stream,
                &state,
                opening_publication,
                &ResponseEnvelope::failure(&auth_request, error),
            )
            .await?;
            return Ok(());
        }
    };
    let authenticated_response = ResponseEnvelope::success(&auth_request, &state.hello())?;
    response_drain::write(
        &mut stream,
        &state,
        opening_publication,
        &authenticated_response,
    )
    .await?;

    rpc::serve(stream, state, controller).await
}

pub(super) async fn recv_after_barrier(
    receiver: &mut broadcast::Receiver<WorkerEvent>,
    barrier_sequence: u64,
) -> std::result::Result<WorkerEvent, broadcast::error::RecvError> {
    loop {
        let event = receiver.recv().await?;
        if matches!(
            &event,
            WorkerEvent::Output(chunk) if chunk.sequence <= barrier_sequence
        ) {
            continue;
        }
        return Ok(event);
    }
}
