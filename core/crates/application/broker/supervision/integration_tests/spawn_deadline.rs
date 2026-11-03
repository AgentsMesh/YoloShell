use crate::fixture_constants as fixture;
use crate::{ChildLaunch, CredentialSource, DetachedSpawner};
use std::{fs, os::unix::fs::PermissionsExt, time::Duration};

const EXPIRING_DEADLINE_LEAD: Duration = Duration::from_millis(20);
const EXPIRING_DEADLINE_WAIT: Duration = Duration::from_millis(40);
const MARKER_FILE: &str = "expired-spawn-marker";
const SCRIPT_FILE: &str = "expired-spawn.sh";
const EXPIRED_DETAIL: &str = "broker launch deadline has expired";

#[test]
fn expired_encoded_deadline_prevents_child_spawn_and_supervision_state_creation() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join(MARKER_FILE);
    let executable = root.path().join(SCRIPT_FILE);
    fs::write(
        &executable,
        format!("#!/bin/sh\nprintf spawned > '{}'\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(
        &executable,
        fs::Permissions::from_mode(fixture::OWNER_EXECUTABLE_MODE),
    )
    .unwrap();
    let spawner = DetachedSpawner::new(
        executable,
        yolo_remote_broker::IdentityPassphrases::none(),
        CredentialSource::None,
    );
    let deadline = yolo_remote_broker::BrokerLaunchDeadline::capture(
        tokio::time::Instant::now() + EXPIRING_DEADLINE_LEAD,
    )
    .unwrap();
    let launch = ChildLaunch::new(
        root.path().join(fixture::CONFIG_FILE),
        root.path().join(fixture::STATE_DIRECTORY),
        fixture::TEST_CONFIG_REVISION.into(),
        fixture::TEST_STATE_ANCHOR.parse().unwrap(),
        uuid::Uuid::nil().to_string().parse().unwrap(),
        deadline,
    )
    .unwrap();
    std::thread::sleep(EXPIRING_DEADLINE_WAIT);
    let error = spawner.spawn(&launch, None).err().unwrap();
    assert!(error.to_string().contains(EXPIRED_DETAIL));
    assert!(!marker.exists());
    assert!(!launch.state_dir().exists());
}
