#[path = "../test_support/build_host.rs"]
mod fixture;
use crate::{ClientError, protocol::method};

#[tokio::test]
async fn build_negotiation_stays_on_the_authenticated_connection() {
    for managed in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let server = fixture::Server::start(home.path(), true, true).await;
        let client = server.client(managed.then_some(home.path()));
        assert_eq!(client.build_image().await.unwrap(), fixture::image());
        assert_eq!(
            server.finish().await,
            vec![method::HOST_STATUS, method::HOST_BUILD]
        );
    }
}

#[tokio::test]
async fn missing_capability_or_wrong_home_never_sends_the_build_query() {
    let home = tempfile::tempdir().unwrap();
    let server = fixture::Server::start(home.path(), false, true).await;
    let client = server.client(Some(home.path()));
    assert!(matches!(
        client.build_image().await,
        Err(ClientError::HostCapabilityUnavailable { .. })
    ));
    assert_eq!(server.finish().await, vec![method::HOST_STATUS]);
    let other = tempfile::tempdir().unwrap();
    let server = fixture::Server::start(other.path(), true, true).await;
    let client = server.client(Some(home.path()));
    assert!(matches!(
        client.build_image().await,
        Err(ClientError::HostHomeMismatch { .. })
    ));
    assert_eq!(server.finish().await, vec![method::HOST_STATUS]);
}

#[tokio::test]
async fn malformed_build_evidence_is_never_success() {
    let home = tempfile::tempdir().unwrap();
    let server = fixture::Server::start(home.path(), true, false).await;
    assert!(
        server
            .client(Some(home.path()))
            .build_image()
            .await
            .is_err()
    );
    assert_eq!(
        server.finish().await,
        vec![method::HOST_STATUS, method::HOST_BUILD]
    );
}
