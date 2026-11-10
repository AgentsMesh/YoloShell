mod boundaries;
mod decode;
#[path = "../test_support/fixtures.rs"]
mod fixtures;
use crate::*;

#[test]
fn requests_bind_every_profile_connection_and_peer_field() {
    let request = fixtures::request();
    request.validate().unwrap();
    let opened = BrokerHubOpened {
        request: request.clone(),
        channel_id: fixtures::CHANNEL,
    };
    opened.validate(&request).unwrap();
    let encoded = serde_json::to_vec(&opened).unwrap();
    assert_eq!(
        serde_json::from_slice::<BrokerHubOpened>(&encoded).unwrap(),
        opened
    );
    let mut swapped = request.clone();
    swapped.binding.connection_id = uuid::Uuid::new_v4();
    assert_eq!(
        opened.validate(&swapped),
        Err(BrokerHubError::ChannelMismatch)
    );
    let mut value = serde_json::to_value(&request).unwrap();
    value["unexpected"] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<BrokerHubOpenRequest>(value).is_err());
}

#[test]
fn invalid_profile_binding_and_nonloopback_or_scoped_endpoint_are_rejected() {
    for invalid in ["", "../other", "with space"] {
        let mut profile = fixtures::profile();
        profile.host_id = invalid.into();
        assert_eq!(profile.validate(), Err(BrokerHubError::Profile));
    }
    for invalid in [
        "",
        "abcd",
        "0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
    ] {
        let mut profile = fixtures::profile();
        profile.profile_authority_sha256 = invalid.into();
        assert!(profile.validate().is_err());
        profile = fixtures::profile();
        profile.config_revision = invalid.into();
        assert!(profile.validate().is_err());
    }
    for field in ["instance", "connection", "generation"] {
        let mut binding = fixtures::binding();
        match field {
            "instance" => binding.broker_instance_id = uuid::Uuid::nil(),
            "connection" => binding.connection_id = uuid::Uuid::nil(),
            _ => binding.connection_generation = u64::default(),
        }
        assert_eq!(binding.validate(), Err(BrokerHubError::Binding));
    }
    for endpoint in ["127.0.0.1:0", "192.0.2.1:22", "[::1%2]:22", "[::]:22"] {
        let mut request = fixtures::request();
        request.endpoint = endpoint.parse().unwrap();
        assert_eq!(request.validate(), Err(BrokerHubError::Endpoint));
    }
}

#[test]
fn protocol_errors_have_stable_nonempty_codes() {
    for error in [
        BrokerHubError::Profile,
        BrokerHubError::Binding,
        BrokerHubError::Endpoint,
        BrokerHubError::Mode,
        BrokerHubError::Artifact,
        BrokerHubError::ProfileMismatch,
        BrokerHubError::StaleBinding,
        BrokerHubError::ChannelMismatch,
        BrokerHubError::Capacity,
    ] {
        assert!(!error.code().is_empty());
        assert!(!error.to_string().is_empty());
    }
}
