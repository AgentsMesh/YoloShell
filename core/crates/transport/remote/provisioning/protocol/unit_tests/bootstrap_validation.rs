use yolo_hub_remote_startup_test_support::*;
use yolo_remote_provisioning_protocol::*;

#[test]
fn enrollment_requires_explicit_observation_authorization_and_distinct_identity() {
    let original = request();
    let mut changed = original.fields().clone();
    changed.schema_version += 1;
    assert_eq!(
        RemoteBootstrapRequest::new(changed),
        Err(BootstrapError::Schema)
    );
    let mut changed = original.fields().clone();
    changed.request_id = uuid::Uuid::nil();
    assert_eq!(
        RemoteBootstrapRequest::new(changed),
        Err(BootstrapError::RequestIdentity)
    );
    let mut changed = original.fields().clone();
    changed.consumer = peer(REMOTE_HUB, CONSUMER_KEY);
    assert_eq!(
        RemoteBootstrapRequest::new(changed),
        Err(BootstrapError::SameIdentity)
    );
    let mut changed = original.fields().clone();
    changed.consumer = peer(CONSUMER_HUB, REMOTE_KEY);
    assert_eq!(
        RemoteBootstrapRequest::new(changed),
        Err(BootstrapError::SameIdentity)
    );
    let mut changed = original.fields().clone();
    changed.rights = [yolo_hub_routing::Right::TerminalWrite]
        .into_iter()
        .collect();
    assert_eq!(
        RemoteBootstrapRequest::new(changed),
        Err(BootstrapError::Authorization)
    );
    let mut changed = original.fields().clone();
    changed.remaining_reexports = yolo_hub_delegation::MAX_DELEGATION_DEPTH + 1;
    assert_eq!(
        RemoteBootstrapRequest::new(changed),
        Err(BootstrapError::Authorization)
    );
}

#[test]
fn receipts_reject_invalid_envelope_and_nonloopback_or_unbound_listener() {
    let request = request();
    for endpoint in [
        std::net::SocketAddr::from((std::net::Ipv4Addr::UNSPECIFIED, PORT)),
        std::net::SocketAddr::from((std::net::Ipv6Addr::UNSPECIFIED, PORT)),
        std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, 0)),
        std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
            std::net::Ipv6Addr::LOCALHOST,
            PORT,
            0,
            1,
        )),
        std::net::SocketAddr::V6(std::net::SocketAddrV6::new(
            std::net::Ipv6Addr::LOCALHOST,
            PORT,
            1,
            0,
        )),
    ] {
        let mut changed = receipt(&request);
        changed.endpoint = endpoint;
        assert_eq!(
            RemoteBootstrapReceipt::new(changed),
            Err(BootstrapError::Endpoint)
        );
    }
    let mut changed = receipt(&request);
    changed.schema_version += 1;
    assert_eq!(
        RemoteBootstrapReceipt::new(changed),
        Err(BootstrapError::Schema)
    );
    let mut changed = receipt(&request);
    changed.request_id = uuid::Uuid::nil();
    assert_eq!(
        RemoteBootstrapReceipt::new(changed),
        Err(BootstrapError::RequestIdentity)
    );
    let mut changed = receipt(&request);
    changed.configuration_revision = 0;
    assert_eq!(
        RemoteBootstrapReceipt::new(changed),
        Err(BootstrapError::Revision)
    );
    for digest in [BAD_DIGEST.to_owned(), RUNTIME_DIGEST.to_uppercase()] {
        let mut changed = receipt(&request);
        changed.bundle_id = digest;
        assert_eq!(
            RemoteBootstrapReceipt::new(changed),
            Err(BootstrapError::Correlation)
        );
    }
}
