use super::*;

#[test]
fn closed_observation_is_typed_until_the_request_error_boundary() {
    for error in [
        WorkerRpcError::from_rpc(yolo_capnp_rpc::Error::Closed),
        WorkerRpcError::from_protocol(ProtocolError::EndOfStream),
    ] {
        assert!(matches!(error, WorkerRpcError::Closed));
        assert!(error.is_transport());
        let body = error.into_error_body();
        assert_eq!(body.code, crate::protocol::error_code::WORKER_UNREACHABLE);
        assert!(body.retryable);
    }
}

#[test]
fn uncertain_confirmation_survives_adapters_as_nonretryable_domain_unknown() {
    for error in [
        WorkerRpcError::from_rpc(yolo_capnp_rpc::Error::OutcomeUnknown),
        WorkerRpcError::from_protocol(ProtocolError::OutcomeUnknown),
    ] {
        assert!(matches!(error, WorkerRpcError::OutcomeUnknown));
        assert!(!error.is_transport());
        let body = error.into_error_body();
        assert_eq!(body.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
        assert!(!body.retryable);
    }
}
