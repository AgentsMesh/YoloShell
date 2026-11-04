use super::*;

pub(super) async fn dispatch(
    pool: &BrokerConnectionPool,
    config: &BrokerConfig,
    envelope: BrokerRequestEnvelope,
    adopted: bool,
) -> DispatchOutcome {
    if envelope.protocol_version != BROKER_PROTOCOL_VERSION {
        return DispatchOutcome::response(BrokerResponse::failure(
            envelope.request_id,
            INVALID_REQUEST_ERROR_CODE,
            BROKER_PROTOCOL_UNSUPPORTED_MESSAGE,
        ));
    }
    match &envelope.request {
        BrokerRequest::Shutdown {
            expected_revision,
            expected_instance_id,
        } => {
            return shutdown_response(
                envelope.request_id,
                pool,
                config,
                expected_revision,
                *expected_instance_id,
                BrokerControl::Rotate,
            );
        }
        BrokerRequest::Retire {
            expected_revision,
            expected_instance_id,
        } => {
            return shutdown_response(
                envelope.request_id,
                pool,
                config,
                expected_revision,
                *expected_instance_id,
                BrokerControl::Retire,
            );
        }
        _ => {}
    }
    let result: anyhow::Result<BrokerResult> = match envelope.request {
        BrokerRequest::Status => {
            let mut status = pool.status().await;
            status.adopted = adopted;
            Ok(BrokerResult::Status(status))
        }
        BrokerRequest::PrepareHubTransport(expected) => pool
            .prepare_hub_transport(&expected)
            .await
            .map(BrokerResult::HubTransport),
        BrokerRequest::OpenHubPeer(_) => Err(yolo_broker_hub_protocol::BrokerHubError::Mode.into()),
        BrokerRequest::ProvisionHub(_)
        | BrokerRequest::RecoverHubProvision(_)
        | BrokerRequest::AdvanceHubProvision(_) => {
            Err(yolo_broker_hub_protocol::BrokerHubError::Mode.into())
        }
        BrokerRequest::Bootstrap => pool
            .bootstrap()
            .await
            .map(|mut status| {
                status.adopted = adopted;
                BrokerResult::Bootstrap(status)
            })
            .map_err(Into::into),
        BrokerRequest::WorkingDirectory => pool.working_directory().await.map_err(Into::into),
        BrokerRequest::Command(request) => pool
            .run_command(&config.command_policy, request)
            .await
            .map_err(Into::into),
        BrokerRequest::FileRead(request) => pool
            .read_file(&config.file_policy, request)
            .await
            .map_err(Into::into),
        BrokerRequest::Shutdown { .. } | BrokerRequest::Retire { .. } => unreachable!(),
    };
    DispatchOutcome::response(match result {
        Ok(value) => BrokerResponse::success(envelope.request_id, value),
        Err(error) => super::super::hub_peer_response::failure(envelope.request_id, &error),
    })
}
