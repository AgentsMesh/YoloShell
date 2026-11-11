use super::address_constants::*;
use std::{
    net::{IpAddr, Ipv6Addr, SocketAddr, SocketAddrV6},
    num::NonZeroU16,
};
use yolo_hub_federation_settings::{DialTransport, MAX_HOST_BYTES, PeerAddress, SshProfileRef};

fn profile() -> SshProfileRef {
    SshProfileRef::new(PROFILE, DIGEST_CHARACTER.repeat(DIGEST_BYTES)).unwrap()
}
fn port() -> NonZeroU16 {
    NonZeroU16::new(PORT).unwrap()
}

#[test]
fn explicit_ssh_reference_roundtrips_and_requires_a_canonical_loopback_endpoint() {
    for host in [LOOPBACK, V6_LOOPBACK] {
        let endpoint = SocketAddr::new(host.parse().unwrap(), PORT);
        let address = PeerAddress::ssh(profile(), endpoint).unwrap();
        assert_eq!(address.ssh_endpoint().unwrap(), endpoint);
        let decoded: PeerAddress =
            serde_json::from_value(serde_json::to_value(&address).unwrap()).unwrap();
        assert_eq!(decoded, address);
        assert!(matches!(address.transport, DialTransport::Ssh { .. }));
    }
    for host in INVALID_SSH_HOSTS {
        let address = PeerAddress {
            host: (*host).into(),
            port: port(),
            transport: DialTransport::Ssh { profile: profile() },
        };
        assert!(address.validate().is_err());
    }
    for endpoint in [
        SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 0),
        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, PORT, PORT.into(), 0)),
        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, PORT, 0, PORT.into())),
    ] {
        assert!(PeerAddress::ssh(profile(), endpoint).is_err());
    }
}

#[test]
fn tcp_requires_a_valid_host_and_never_produces_an_ssh_endpoint() {
    let address = PeerAddress::tcp(PROFILE, port());
    address.validate().unwrap();
    assert!(address.ssh_endpoint().is_err());
    for host in INVALID_HOSTS {
        assert!(PeerAddress::tcp(*host, port()).validate().is_err());
    }
    assert!(
        PeerAddress::tcp(DIGEST_CHARACTER.repeat(MAX_HOST_BYTES), port())
            .validate()
            .is_ok()
    );
    assert!(
        PeerAddress::tcp(PROFILE.repeat(MAX_HOST_BYTES), port())
            .validate()
            .is_err()
    );
}

#[test]
fn transport_tags_cannot_silently_drop_ssh_profile_binding() {
    let address = PeerAddress::tcp(LOOPBACK, port());
    let mut value = serde_json::to_value(&address).unwrap();
    value.as_object_mut().unwrap().remove(TRANSPORT);
    assert!(serde_json::from_value::<PeerAddress>(value).is_err());
    let mut value = serde_json::to_value(&address).unwrap();
    value[TRANSPORT][PROFILE_FIELD] = serde_json::to_value(profile()).unwrap();
    assert!(serde_json::from_value::<PeerAddress>(value).is_err());
    let ssh =
        PeerAddress::ssh(profile(), SocketAddr::new(LOOPBACK.parse().unwrap(), PORT)).unwrap();
    let mut value = serde_json::to_value(&ssh).unwrap();
    value[TRANSPORT]
        .as_object_mut()
        .unwrap()
        .remove(PROFILE_FIELD);
    assert!(serde_json::from_value::<PeerAddress>(value).is_err());
    let mut value = serde_json::to_value(&ssh).unwrap();
    value[TRANSPORT][KIND] = UNKNOWN_KIND.into();
    assert!(serde_json::from_value::<PeerAddress>(value).is_err());
}
