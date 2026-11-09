use super::*;
use yolo_broker_supervision_state::{BrokerReadiness, ChildLaunch};
use yolo_process::ProcessIdentity;
use yolo_remote_broker::BrokerPaths;

#[path = "supervised_wait_child.rs"]
mod child;
use child::OwnedChild;

const CHILD_LIFETIME: &str = "30";
const CONFIG_FILE: &str = "config.json";
const CONFIG_REVISION: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const BROKER_HOST: &str = "supervised-wait-test-host";

pub(super) struct Fixture {
    pub(super) child: OwnedChild,
    pub(super) active: SupervisionRecord,
    pub(super) ready: SupervisionRecord,
    pub(super) paths: SupervisionPaths,
    _directory: tempfile::TempDir,
}

impl Fixture {
    pub(super) fn new() -> Self {
        Self::after_spawn(|_| {})
    }

    pub(super) fn after_spawn(after_spawn: impl FnOnce(&DetachedChildCustody)) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let state = directory
            .path()
            .join(crate::fixture_constants::STATE_DIRECTORY);
        let broker_paths = BrokerPaths::new(&state).unwrap();
        let options = ChildLaunch::new(
            directory.path().join(CONFIG_FILE),
            state,
            CONFIG_REVISION.into(),
            broker_paths.state_anchor(),
            uuid::Uuid::new_v4().to_string().parse().unwrap(),
            yolo_remote_broker::BrokerLaunchDeadline::capture(
                tokio::time::Instant::now() + crate::fixture_constants::START_TIMEOUT,
            )
            .unwrap(),
        )
        .unwrap();
        let paths = SupervisionPaths::for_launch(&options).unwrap();
        let raw = yolo_process::spawn_waitable_child_before_deadline(
            std::process::Command::new(crate::fixture_constants::TEST_SLEEP_EXECUTABLE)
                .arg(CHILD_LIFETIME)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null()),
            Instant::now() + crate::fixture_constants::START_TIMEOUT,
        )
        .unwrap();
        let child = OwnedChild::capture(raw).unwrap();
        after_spawn(&child.custody);
        let supervisor = child.custody.identity().unwrap();
        let launcher = ProcessIdentity::capture(std::process::id()).unwrap();
        let mut active = SupervisionRecord::intent(&options, supervisor, launcher).unwrap();
        active.activate(supervisor).unwrap();
        let mut ready = active.clone();
        ready
            .ready(&BrokerReadiness {
                instance_id: uuid::Uuid::new_v4(),
                host_id: BROKER_HOST.into(),
                config_revision: CONFIG_REVISION.into(),
            })
            .unwrap();
        Self {
            child,
            active,
            ready,
            paths,
            _directory: directory,
        }
    }
}
