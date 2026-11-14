use super::constants::*;
use crate::*;
use std::{os::unix::fs::PermissionsExt, sync::Arc};
use yolo_hub_state::HubState;
use yolo_remote_broker::{BrokerLaunchDeadline, BrokerPaths};
use yolo_ssh_profile_protocol::SshProfileRef;

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub owner: Arc<HubState>,
    pub scope: HubBrokerLaunchScope,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        std::fs::set_permissions(
            root.path(),
            std::fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
        )
        .unwrap();
        let home = root.path().join("host");
        let owner = Arc::new(HubState::open(&root.path().join("hub"), &home).unwrap());
        let broker = BrokerPaths::new(root.path().join("broker")).unwrap();
        let config = root.path().join("config.json");
        std::fs::write(&config, SOURCE).unwrap();
        std::fs::set_permissions(&config, std::fs::Permissions::from_mode(PRIVATE_MODE)).unwrap();
        let deadline =
            BrokerLaunchDeadline::capture(tokio::time::Instant::now() + SCOPE_LIFETIME).unwrap();
        let scope = HubBrokerLaunchScope::capture(
            &owner,
            &home,
            SshProfileRef::new(PROFILE, REVISION).unwrap(),
            config,
            &broker,
            REVISION.into(),
            deadline,
        )
        .unwrap();
        Self { root, owner, scope }
    }
}
