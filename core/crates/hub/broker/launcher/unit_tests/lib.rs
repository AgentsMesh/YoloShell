use yolo_hub_ssh_profile_protocol_test_support::profile;
use yolo_remote_broker::{BROKER_PROTOCOL_VERSION, BrokerStatus};

#[test]
fn readiness_requires_adoption_exact_profile_and_a_live_connection() {
    let registration = profile(0);
    let ready = BrokerStatus {
        protocol_version: BROKER_PROTOCOL_VERSION,
        adopted: true,
        instance_id: uuid::Uuid::new_v4(),
        host_id: registration.profile.profile_id().into(),
        config_revision: registration.config_revision.clone(),
        connection_id: Some(uuid::Uuid::new_v4()),
        connection_generation: 1,
        capabilities: vec![yolo_broker_hub_protocol::HUB_TRANSPORT_CAPABILITY.into()],
    };
    crate::result::validate(&ready, &registration).unwrap();
    let mut candidates = Vec::new();
    let mut invalid = ready.clone();
    invalid.adopted = false;
    candidates.push(invalid);
    let mut invalid = ready.clone();
    invalid.connection_id = None;
    candidates.push(invalid);
    let mut invalid = ready.clone();
    invalid.connection_id = Some(uuid::Uuid::nil());
    candidates.push(invalid);
    let mut invalid = ready.clone();
    invalid.instance_id = uuid::Uuid::nil();
    candidates.push(invalid);
    let mut invalid = ready.clone();
    invalid.connection_generation = 0;
    candidates.push(invalid);
    let mut invalid = ready.clone();
    invalid.capabilities.clear();
    candidates.push(invalid);
    let mut invalid = ready.clone();
    invalid.host_id = profile(1).profile.profile_id().into();
    candidates.push(invalid);
    let mut invalid = ready.clone();
    invalid.config_revision.clear();
    candidates.push(invalid);
    let mut invalid = ready;
    invalid.protocol_version = u16::MAX;
    candidates.push(invalid);
    for invalid in candidates {
        assert!(crate::result::validate(&invalid, &registration).is_err());
    }
}
