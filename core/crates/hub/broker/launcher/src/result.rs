use crate::constants::*;
use anyhow::{Result, ensure};
use yolo_hub_ssh_profile_protocol::SshBrokerProfile;
use yolo_remote_broker::{BROKER_PROTOCOL_VERSION, BrokerStatus};

pub(crate) fn validate(status: &BrokerStatus, registration: &SshBrokerProfile) -> Result<()> {
    ensure!(
        status.protocol_version == BROKER_PROTOCOL_VERSION
            && status.adopted
            && status.host_id == registration.profile.profile_id()
            && status.config_revision == registration.config_revision
            && !status.instance_id.is_nil()
            && status.connection_id.is_some_and(|id| !id.is_nil())
            && status.connection_generation != 0
            && status.supports(yolo_broker_hub_protocol::HUB_TRANSPORT_CAPABILITY),
        WRONG_BROKER
    );
    Ok(())
}
