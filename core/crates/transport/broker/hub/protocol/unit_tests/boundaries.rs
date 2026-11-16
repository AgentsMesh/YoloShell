use super::fixtures;
use crate::*;
use std::net::{Ipv6Addr, SocketAddr, SocketAddrV6};

const HOST_BYTE: &str = "a";
const EXTRA_BYTE: usize = 1;
const PEER_PORT: u16 = 32123;
const FLOW_INFORMATION: u32 = 1;
const INTERFACE_SCOPE: u32 = 2;

#[test]
fn profile_host_bound_accepts_the_limit_and_rejects_longer_or_path_like_values() {
    let mut profile = fixtures::profile();
    profile.host_id = HOST_BYTE.repeat(crate::constants::MAX_HOST_ID_BYTES);
    profile.validate().unwrap();
    profile.host_id.push_str(HOST_BYTE);
    assert_eq!(
        profile.host_id.len(),
        crate::constants::MAX_HOST_ID_BYTES + EXTRA_BYTE
    );
    assert_eq!(profile.validate(), Err(BrokerHubError::Profile));
    for path in ["/srv/hub", "hub/../other", "hub\0suffix", "hub\nother"] {
        profile.host_id = path.into();
        assert_eq!(profile.validate(), Err(BrokerHubError::Profile));
    }
}

#[test]
fn ipv6_loopback_requires_both_scope_and_flow_information_to_be_zero() {
    let mut request = fixtures::request();
    request.endpoint = SocketAddr::V6(SocketAddrV6::new(
        Ipv6Addr::LOCALHOST,
        PEER_PORT,
        u32::default(),
        u32::default(),
    ));
    request.validate().unwrap();
    let opened = BrokerHubOpened {
        request: request.clone(),
        channel_id: fixtures::CHANNEL,
    };
    opened.validate(&request).unwrap();
    for (flow, scope) in [
        (FLOW_INFORMATION, u32::default()),
        (u32::default(), INTERFACE_SCOPE),
        (FLOW_INFORMATION, INTERFACE_SCOPE),
    ] {
        request.endpoint = SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::LOCALHOST,
            PEER_PORT,
            flow,
            scope,
        ));
        assert_eq!(request.validate(), Err(BrokerHubError::Endpoint));
    }
}

#[test]
fn identical_invalid_acknowledgement_and_request_never_authorize_a_tunnel() {
    let mut request = fixtures::request();
    request.binding.connection_generation = u64::default();
    let opened = BrokerHubOpened {
        request: request.clone(),
        channel_id: fixtures::CHANNEL,
    };
    assert_eq!(opened.validate(&request), Err(BrokerHubError::Binding));
    let mut request = fixtures::request();
    request.binding.profile.config_revision.clear();
    let opened = BrokerHubOpened {
        request: request.clone(),
        channel_id: fixtures::CHANNEL,
    };
    assert_eq!(opened.validate(&request), Err(BrokerHubError::Profile));
}
