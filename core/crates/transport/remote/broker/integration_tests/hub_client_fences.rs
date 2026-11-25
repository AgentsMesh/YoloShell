use super::{
    hub_client_server::{Fault, Server},
    hub_fixture::*,
};
use crate::remote_broker::*;
const CALL_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(100);

#[tokio::test]
async fn client_never_returns_raw_io_for_an_uncorrelated_or_replaced_upgrade() {
    for fault in [
        Fault::Request,
        Fault::Version,
        Fault::Instance,
        Fault::Connection,
        Fault::Generation,
        Fault::Profile,
        Fault::Peer,
        Fault::Endpoint,
        Fault::Result,
        Fault::Namespace,
    ] {
        let server = Server::new(fault);
        let error = BrokerClient::new(&server.path)
            .open_hub_peer(yolo_broker_hub_test_support::request(), deadline())
            .await
            .err()
            .unwrap();
        match fault {
            Fault::Request => assert!(matches!(error, BrokerClientError::RequestMismatch)),
            Fault::Version => assert!(matches!(error, BrokerClientError::IncompatibleProtocol)),
            Fault::Result => assert!(matches!(error, BrokerClientError::UnexpectedResult)),
            Fault::Namespace => assert!(matches!(error, BrokerClientError::UnsafeSocket(_))),
            _ => assert!(matches!(
                error,
                BrokerClientError::HubProtocol(BrokerHubError::ChannelMismatch)
            )),
        }
        server.finish().await;
    }
}

#[tokio::test]
async fn stalled_upgrade_honors_the_callers_absolute_deadline() {
    let server = Server::new(Fault::Deadline);
    let deadline = tokio::time::Instant::now() + CALL_TIMEOUT;
    assert!(matches!(
        BrokerClient::new(&server.path)
            .open_hub_peer(yolo_broker_hub_test_support::request(), deadline)
            .await,
        Err(BrokerClientError::Deadline)
    ));
    server.finish().await;
}
