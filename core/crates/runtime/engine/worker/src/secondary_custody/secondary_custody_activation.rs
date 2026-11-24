use super::secondary_custody_adoption_deadline::WorkerAdoptionDeadline;
use super::{WorkerCustodyLease, WorkerState};
use anyhow::Result;

const ACTIVATION_TIMEOUT_CONTEXT: &str = "Worker PTY custody activation timed out";

pub(crate) async fn activate(lease: WorkerCustodyLease) -> Result<WorkerCustodyLease> {
    activate_before(lease, WorkerAdoptionDeadline::production()).await
}

pub(crate) async fn activate_before(
    lease: WorkerCustodyLease,
    deadline: WorkerAdoptionDeadline,
) -> Result<WorkerCustodyLease> {
    deadline
        .wait(lease.activate(), ACTIVATION_TIMEOUT_CONTEXT)
        .await?
}

pub(crate) async fn activate_successor_before(
    lease: WorkerCustodyLease,
    deadline: WorkerAdoptionDeadline,
    state: &WorkerState,
) -> Result<WorkerCustodyLease> {
    #[cfg(test)]
    state
        .control_test_hooks
        .custody_before_activation
        .pause_if_armed()
        .await;
    #[cfg(not(test))]
    let _ = state;
    activate_before(lease, deadline).await
}
