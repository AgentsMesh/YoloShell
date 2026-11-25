pub mod constants;
use constants::*;
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_ssh_profile_protocol::*;

pub fn identity() -> PeerIdentity {
    yolo_hub_delegation_test_support::Identity::new().identity
}
pub fn profile(index: usize) -> SshBrokerProfile {
    SshBrokerProfile {
        launch: None,
        profile: SshProfileRef::new(format!("{PROFILE_PREFIX}{index}"), REVISION_A).unwrap(),
        broker_state: format!("{BROKER_PREFIX}{index}").into(),
        config_revision: CONFIG_REVISION.into(),
    }
}
pub fn configuration() -> SshProfileConfiguration {
    SshProfileConfiguration {
        profiles: vec![profile(0)],
        ..Default::default()
    }
}
pub fn state() -> SshProfileState {
    SshProfileState {
        hub: identity(),
        revision: FIRST_SSH_PROFILE_REVISION,
        last_change: Uuid::new_v4(),
        configuration: configuration(),
    }
}
pub fn change(
    previous: &SshProfileState,
    configuration: SshProfileConfiguration,
) -> SshProfileChange {
    SshProfileChange {
        request_id: Uuid::new_v4(),
        expected_revision: previous.revision,
        configuration,
    }
}
