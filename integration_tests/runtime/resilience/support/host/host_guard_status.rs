use super::*;
use anyhow::{Result, ensure};
use yolo_process_test_support::ProcessIdentityObservation;
use yolo_runtime::model::HostStatus;

const HOST_STATUS_HOME_MISMATCH: &str = "ready Host reported another home";
const HOST_STATUS_PID_MISMATCH: &str = "ready status belongs to another Host process";
const HOST_STATUS_IDENTITY_MISMATCH: &str = "guarded Host identity changed before readiness";

impl HostGuard {
    pub(crate) fn verify_ready_status(&self, status: &HostStatus) -> Result<()> {
        ensure!(
            status.home == self.expected_home,
            "{HOST_STATUS_HOME_MISMATCH}: expected {}, received {}",
            self.expected_home.display(),
            status.home.display()
        );
        ensure!(
            status.host_pid == self.identity.pid,
            "{HOST_STATUS_PID_MISMATCH}: expected {}, received {}",
            self.identity.pid,
            status.host_pid
        );
        ensure!(
            self.identity.observe() == ProcessIdentityObservation::Matching,
            HOST_STATUS_IDENTITY_MISMATCH
        );
        Ok(())
    }
}
