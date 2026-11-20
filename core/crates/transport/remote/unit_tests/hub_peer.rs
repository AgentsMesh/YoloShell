use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::HubId;
use yolo_remote::{HubPeerEndpoint, HubPeerRequest, RemoteError};

const KEY_BYTES: usize = 32;
const KEY_FILL: u8 = 7;
const SOCKET: &str = "/run/user/1000/yolo/hub/peer.sock";

#[test]
fn hub_peer_targets_accept_only_absolute_canonical_socket_paths() {
    assert_eq!(
        HubPeerEndpoint::new(SOCKET).unwrap().path(),
        std::path::Path::new(SOCKET)
    );
    for invalid in [
        "",
        "peer.sock",
        "relative/peer.sock",
        "/tmp/../peer.sock",
        "/tmp/./peer.sock",
        "/tmp/peer\n.sock",
        "/tmp/peer\0.sock",
    ] {
        assert!(HubPeerEndpoint::new(invalid).is_err(), "{invalid:?}");
    }
    let long = format!("/{}", "a".repeat(100));
    assert!(HubPeerEndpoint::new(long).is_err());
}

#[test]
fn a_scoped_hub_peer_request_captures_transport_and_inner_peer_identity() {
    let endpoint = HubPeerEndpoint::new(SOCKET).unwrap();
    let peer = PeerIdentity::new(HubId::new(), [KEY_FILL; KEY_BYTES]).unwrap();
    assert!(matches!(
        HubPeerRequest::new(uuid::Uuid::nil(), endpoint.clone(), peer),
        Err(RemoteError::ChannelMismatch)
    ));
    let connection = uuid::Uuid::new_v4();
    let request = HubPeerRequest::new(connection, endpoint.clone(), peer).unwrap();
    assert_eq!(request.connection_id(), connection);
    assert_eq!(request.endpoint(), &endpoint);
    assert_eq!(request.peer(), peer);
}
