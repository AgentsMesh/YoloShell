use super::fixture;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use yolo_remote_broker::{
    BrokerLaunchDeadline, BrokerPaths, BrokerReconcileGuard, authorize_broker_launch,
};
use yolo_runtime::cli::constants;

pub(crate) struct BrokerProcess {
    child: Child,
}

impl BrokerProcess {
    pub(crate) async fn start(
        binary: &Path,
        config: &Path,
        state: &Path,
        expected_revision: &str,
    ) -> Self {
        let paths = BrokerPaths::new(state).unwrap();
        let launch_deadline = tokio::time::Instant::now() + fixture::START_TIMEOUT;
        let guard = BrokerReconcileGuard::acquire(&paths, launch_deadline)
            .await
            .unwrap();
        let authority = authorize_broker_launch(&paths, &guard, launch_deadline)
            .await
            .unwrap();
        let encoded_deadline = BrokerLaunchDeadline::capture(launch_deadline).unwrap();
        drop(guard);
        let child = Command::new(binary)
            .args(yolo_launch_contract::ProcessRole::Remote.arguments())
            .arg(constants::SERVE_COMMAND)
            .arg(constants::CONFIG_ARGUMENT)
            .arg(config)
            .arg(constants::STATE_DIRECTORY_ARGUMENT)
            .arg(state)
            .arg(constants::EXPECTED_REVISION_ARGUMENT)
            .arg(expected_revision)
            .arg(constants::EXPECTED_ANCHOR_ARGUMENT)
            .arg(paths.state_anchor().to_string())
            .arg(constants::EXPECTED_AUTHORITY_ARGUMENT)
            .arg(authority.to_string())
            .arg(constants::EXPECTED_STARTUP_DEADLINE_ARGUMENT)
            .arg(encoded_deadline.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        Self { child }
    }

    pub(crate) fn is_running(&mut self) -> bool {
        self.child.try_wait().unwrap().is_none()
    }

    pub(crate) fn child_id(&self) -> u32 {
        self.child.id()
    }

    pub(crate) fn kill_and_wait(&mut self) {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
    }

    pub(crate) async fn wait_for_exit(&mut self) {
        let deadline = tokio::time::Instant::now() + fixture::PROCESS_WAIT_TIMEOUT;
        while tokio::time::Instant::now() < deadline {
            if !self.is_running() {
                return;
            }
            tokio::time::sleep(fixture::PROCESS_WAIT_INTERVAL).await;
        }
        if !self.is_running() {
            return;
        }
        panic!("broker process did not exit");
    }
}

impl Drop for BrokerProcess {
    fn drop(&mut self) {
        if self.is_running() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
