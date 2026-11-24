use super::secondary_custody::{RecoverySnapshot, require_gone_runtime_processes};
use super::{Host, RuntimeRecord};
use crate::process::linux_containment::{
    LinuxContainmentError, LinuxContainmentRecovery, LinuxContainmentRecoveryPoll,
};
use crate::pty_custody::CustodyRuntimeIdentity;
use anyhow::{Result, ensure};
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

mod publication;
pub(in crate::host) mod receipt;
mod snapshot;

const PENDING_DETAIL: &str = "Linux runtime retirement is awaiting exact containment settlement";
const RUNTIME_DETAIL: &str = "Linux runtime retirement evidence changed";

#[derive(Debug)]
pub(super) struct Pending;

impl std::fmt::Display for Pending {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(PENDING_DETAIL)
    }
}

impl std::error::Error for Pending {}

pub(super) async fn settle_after_lifecycle_lock(
    host: &Host,
    runtime: &RuntimeRecord,
) -> Result<()> {
    let guard = SessionDurableMutationGuard::acquire_after_lifecycle_lock(
        &host.paths,
        &host.namespace,
        runtime.terminal_id,
    )?;
    guard.require_no_pending_intent()?;
    let snapshot = RecoverySnapshot::capture(host, &CustodyRuntimeIdentity::from_record(runtime))?;
    ensure!(snapshot.runtime == *runtime, RUNTIME_DETAIL);
    receipt::validate_lease(host, &snapshot)?;
    ensure!(
        super::process_identity_is_gone(runtime.worker_pid, runtime.worker_start_time),
        RUNTIME_DETAIL
    );
    if snapshot.custody.retired_lease().is_some() {
        snapshot.validate_receipt(host)?;
        let proof = receipt::RetirementProof::capture(host, &snapshot)?
            .ok_or_else(|| anyhow::Error::new(Pending))?;
        return publication::confirm(host, &guard, &snapshot, &proof);
    }
    let containment = snapshot::ContainmentSnapshot::capture(host, runtime)?;
    let mut recovery = LinuxContainmentRecovery::open(
        containment.record.clone(),
        runtime.terminal_id,
        runtime.runtime_epoch,
        snapshot::root(runtime),
    )?;
    let deadline =
        tokio::time::Instant::now() + crate::session_timing::LINUX_CONTAINMENT_RECOVERY_TIMEOUT;
    loop {
        guard.require_no_pending_intent()?;
        snapshot.verify(host)?;
        containment.verify(host, runtime)?;
        match recovery.poll() {
            Ok(LinuxContainmentRecoveryPoll::Settled { .. }) => break,
            Ok(LinuxContainmentRecoveryPoll::Pending) => {}
            Err(LinuxContainmentError::Unavailable(_)) => {
                return Err(anyhow::Error::new(Pending));
            }
            Err(error) => return Err(error.into()),
        }
        let now = tokio::time::Instant::now();
        if now >= deadline {
            return Err(anyhow::Error::new(Pending));
        }
        tokio::time::sleep_until(deadline.min(now + super::PROCESS_EXIT_POLL_INTERVAL)).await;
    }
    let proof = receipt::RetirementProof::from_settlement(containment, recovery.into_settlement()?);
    require_gone_runtime_processes(runtime)?;
    publication::publish(host, &guard, &snapshot, &proof)
}
