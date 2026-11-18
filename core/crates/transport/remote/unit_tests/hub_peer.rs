use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV6};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_identity::HubId;
use yolo_remote::{HubPeerEndpoint, HubPeerRequest, RemoteError};

const PORT: u16 = 42323;
const ZERO_PORT: u16 = 0;
const SCOPE: u32 = 1;
const KEY_BYTES: usize = 32;
const KEY_FILL: u8 = 7;

#[test]
fn hub_peer_targets_accept_only_nonzero_unscoped_ip_loopback_endpoints() {
    for address in [
        IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(Ipv6Addr::LOCALHOST),
    ] {
        let socket = SocketAddr::new(address, PORT);
        assert_eq!(HubPeerEndpoint::new(socket).unwrap().address(), socket);
        assert!(HubPeerEndpoint::new(SocketAddr::new(address, ZERO_PORT)).is_err());
    }
    for address in [
        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        IpAddr::V4(Ipv4Addr::BROADCAST),
        IpAddr::V6(Ipv6Addr::UNSPECIFIED),
    ] {
        assert!(HubPeerEndpoint::new(SocketAddr::new(address, PORT)).is_err());
    }
    for (flow, scope) in [(SCOPE, u32::default()), (u32::default(), SCOPE)] {
        let address = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, PORT, flow, scope));
        assert!(HubPeerEndpoint::new(address).is_err());
    }
}

#[test]
fn a_scoped_hub_peer_request_captures_transport_and_inner_peer_identity() {
    let endpoint = HubPeerEndpoint::new(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), PORT)).unwrap();
    let peer = PeerIdentity::new(HubId::new(), [KEY_FILL; KEY_BYTES]).unwrap();
    assert!(matches!(
        HubPeerRequest::new(uuid::Uuid::nil(), endpoint, peer),
        Err(RemoteError::ChannelMismatch)
    ));
    let connection = uuid::Uuid::new_v4();
    let request = HubPeerRequest::new(connection, endpoint, peer).unwrap();
    assert_eq!(request.connection_id(), connection);
    assert_eq!(request.endpoint(), endpoint);
    assert_eq!(request.peer(), peer);
}
