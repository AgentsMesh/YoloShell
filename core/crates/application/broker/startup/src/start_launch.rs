use crate::StartRequest;
use anyhow::Result;
use yolo_broker_supervision_state::ChildLaunch;
use yolo_remote_broker::{
    BrokerConfigGeneration, BrokerLaunchAuthority, BrokerLaunchDeadline, BrokerPaths,
};

pub(super) fn options(
    requested: &StartRequest,
    generation: &BrokerConfigGeneration,
    paths: &BrokerPaths,
    authority: BrokerLaunchAuthority,
    deadline: tokio::time::Instant,
) -> Result<ChildLaunch> {
    ChildLaunch::new(
        generation.config_path().to_owned(),
        requested.broker_state.clone(),
        generation.revision().to_owned(),
        paths.state_anchor(),
        authority,
        BrokerLaunchDeadline::capture(deadline)?,
    )
}
