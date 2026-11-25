fn shutdown_response(
    id: uuid::Uuid,
    pool: &BrokerConnectionPool,
    config: &BrokerConfig,
    expected_revision: &str,
    expected_instance_id: uuid::Uuid,
    control: BrokerControl,
) -> DispatchOutcome {
    if expected_instance_id != pool.instance_id() {
        return DispatchOutcome::response(BrokerResponse::failure(
            id,
            INVALID_REQUEST_ERROR_CODE,
            BROKER_SHUTDOWN_INSTANCE_CHANGED_MESSAGE,
        ));
    }
    if expected_revision != config.revision() {
        return DispatchOutcome::response(BrokerResponse::failure(
            id,
            INVALID_REQUEST_ERROR_CODE,
            BROKER_SHUTDOWN_REVISION_CHANGED_MESSAGE,
        ));
    }
    let result = match control {
        BrokerControl::Rotate => BrokerResult::Shutdown {
            config_revision: config.revision().into(),
            instance_id: pool.instance_id(),
        },
        BrokerControl::Retire => BrokerResult::Retire {
            config_revision: config.revision().into(),
            instance_id: pool.instance_id(),
        },
        BrokerControl::Continue => unreachable!(),
    };
    DispatchOutcome {
        response: BrokerResponse::success(id, result),
        control,
    }
}
