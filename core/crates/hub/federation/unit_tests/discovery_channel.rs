use super::classify_connection;
use yolo_hub_peer_rpc::RpcError;

#[test]
fn failed_connection_does_not_become_a_successful_discovery_deferral() {
    for failure in [
        RpcError::Closed,
        RpcError::Deadline,
        RpcError::Invalid,
        RpcError::Runtime,
        RpcError::OutcomeUnknown,
        // An identical diagnostic from a remote peer conveys no local admission proof.
        RpcError::Remote(RpcError::Capacity.to_string()),
    ] {
        let error = classify_connection(Err(failure.clone())).err().unwrap();
        assert_eq!(error.downcast_ref::<RpcError>(), Some(&failure));
    }
}

#[test]
fn only_typed_local_capacity_allows_the_existing_link_to_continue() {
    assert!(
        classify_connection(Err(RpcError::Capacity))
            .unwrap()
            .is_none()
    );
}
