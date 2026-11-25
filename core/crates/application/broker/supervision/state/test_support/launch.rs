use crate::{ChildLaunch, fixture_constants::*};
use yolo_remote_broker::{BrokerLaunchDeadline, BrokerPaths};

pub fn launch() -> ChildLaunch {
    ChildLaunch::new(
        CONFIG_PATH.into(),
        STATE_PATH.into(),
        CONFIG_REVISION.into(),
        STATE_ANCHOR.parse().unwrap(),
        uuid::Uuid::new_v4().to_string().parse().unwrap(),
        deadline(),
    )
    .unwrap()
}

pub fn for_paths(paths: &BrokerPaths, authority: uuid::Uuid) -> ChildLaunch {
    ChildLaunch::new(
        CONFIG_PATH.into(),
        paths.state_dir().to_owned(),
        CONFIG_REVISION.into(),
        paths.state_anchor(),
        authority.to_string().parse().unwrap(),
        deadline(),
    )
    .unwrap()
}

pub fn with_authority(launch: &ChildLaunch, authority: uuid::Uuid) -> ChildLaunch {
    ChildLaunch::new(
        launch.config().to_owned(),
        launch.state_dir().to_owned(),
        launch.config_revision().into(),
        launch.state_anchor(),
        authority.to_string().parse().unwrap(),
        launch.startup_deadline().clone(),
    )
    .unwrap()
}

pub fn deadline() -> BrokerLaunchDeadline {
    BrokerLaunchDeadline::capture(tokio::time::Instant::now() + START_TIMEOUT).unwrap()
}
