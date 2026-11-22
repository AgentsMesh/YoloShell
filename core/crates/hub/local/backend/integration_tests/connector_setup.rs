use super::{cleanup_rpc::ProtocolFixture, connectors as support, fixtures};
use yolo_hub_connector_protocol::ConnectorError;
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_identity::HubId;
use yolo_hub_terminal_protocol::PublicError;
use yolo_paths::directory::PrivateDirectory;

const TEST_PEER_KEY_BYTE: u8 = 47;

#[tokio::test]
async fn disabled_connector_registry_and_foreign_identity_cannot_issue_host_requests() {
    let fixture = ProtocolFixture::new();
    assert_eq!(
        fixture
            .without_request(
                fixture
                    .backend
                    .register_connector(uuid::Uuid::new_v4(), support::scope(fixtures::target()))
            )
            .await
            .unwrap_err(),
        ConnectorError::Unavailable
    );
    assert_eq!(
        fixture
            .without_request(fixture.backend.list_connectors())
            .await,
        Err(ConnectorError::Unavailable)
    );
    assert_eq!(
        fixture
            .without_request(fixture.backend.disconnect_all_connectors())
            .await,
        Err(ConnectorError::Unavailable)
    );
    let root = tempfile::tempdir().unwrap();
    let directory = PrivateDirectory::ensure(&root.path().canonicalize().unwrap()).unwrap();
    let identity =
        PeerIdentity::new(HubId::new(), [TEST_PEER_KEY_BYTE; ED25519_PUBLIC_KEY_BYTES]).unwrap();
    assert!(matches!(
        fixture.backend.clone().with_connectors(directory, identity),
        Err(PublicError::OriginMismatch)
    ));
}
