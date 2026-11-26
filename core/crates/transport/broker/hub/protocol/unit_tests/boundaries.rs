use super::fixtures;
use crate::*;

const HOST_BYTE: &str = "a";
const EXTRA_BYTE: usize = 1;
const PATH_BYTE: &str = "a";
const MAX_SOCKET_BYTES: usize = 100;

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
fn socket_budget_accepts_the_kernel_limit_and_rejects_longer_paths() {
    let mut request = fixtures::request();
    request.socket = format!("/{}", PATH_BYTE.repeat(MAX_SOCKET_BYTES - 1)).into();
    request.validate().unwrap();
    request.socket = format!("/{}", PATH_BYTE.repeat(MAX_SOCKET_BYTES)).into();
    assert_eq!(request.validate(), Err(BrokerHubError::Endpoint));
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
