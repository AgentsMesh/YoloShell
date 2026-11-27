use super::DurableProcessIdentity;
use anyhow::Result;
use std::path::Path;

pub struct WorkerTerminationClaim {
    worker: DurableProcessIdentity,
}

pub fn begin_worker_termination(
    settings_directory: &Path,
    worker: &DurableProcessIdentity,
) -> Result<WorkerTerminationClaim> {
    let ledger_path = super::path(settings_directory)?;
    super::persistence::update(&ledger_path, |ledger| {
        super::state::begin_worker_termination(ledger, worker)
    })?;
    Ok(WorkerTerminationClaim {
        worker: worker.clone(),
    })
}

pub fn settle_worker_termination(
    settings_directory: &Path,
    claim: &WorkerTerminationClaim,
) -> Result<()> {
    let ledger_path = super::path(settings_directory)?;
    super::persistence::update(&ledger_path, |ledger| {
        super::state::settle_worker_termination(ledger, &claim.worker)
    })
}
