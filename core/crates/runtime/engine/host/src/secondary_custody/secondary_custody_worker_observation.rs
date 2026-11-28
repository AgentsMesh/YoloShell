use super::super::SecondaryCustodyLease;
use crate::process::{BootIdentityRelation, ProcessIdentity, ProcessIdentityObservation};

pub(super) async fn observe_worker_after_disconnect(
    lease: &SecondaryCustodyLease,
) -> ProcessIdentityObservation {
    let deadline = tokio::time::Instant::now()
        + crate::session_timing::PROCESS_CAPABILITY_EXIT_EVENT_CONVERGENCE_TIMEOUT;
    loop {
        let observation = observe_worker(lease);
        if observation != ProcessIdentityObservation::Matching {
            return observation;
        }
        let now = tokio::time::Instant::now();
        if now >= deadline {
            return observation;
        }
        tokio::time::sleep_until(std::cmp::min(
            deadline,
            now + crate::host::PROCESS_EXIT_POLL_INTERVAL,
        ))
        .await;
    }
}

#[cfg(target_os = "linux")]
pub(in super::super) async fn wait_for_worker_exit(lease_ref: &std::sync::Weak<SecondaryCustodyLease>) {
    let Some(lease) = lease_ref.upgrade() else { return };
    if crate::process::boot_identity_relation(&lease.runtime.boot_id, &lease.host_boot_id)
        != BootIdentityRelation::Same { return; }
    let process = worker_process(&lease);
    drop(lease);
    let _ = process.wait_for_linux_exit().await;
}

pub(in super::super) fn worker_is_proven_gone(lease: &SecondaryCustodyLease) -> bool {
    matches!(observe_worker(lease), ProcessIdentityObservation::Gone)
}

pub(in super::super) async fn worker_is_proven_gone_after_disconnect(
    lease: &SecondaryCustodyLease,
) -> bool {
    matches!(
        observe_worker_after_disconnect(lease).await,
        ProcessIdentityObservation::Gone
    )
}

pub(super) fn observe_worker(lease: &SecondaryCustodyLease) -> ProcessIdentityObservation {
    worker_process(lease).observe_on_boot(&lease.runtime.boot_id, &lease.host_boot_id)
}

fn worker_process(lease: &SecondaryCustodyLease) -> ProcessIdentity {
    #[cfg(test)]
    if let Some(identity) = *crate::host::lock_recover(&lease.worker_identity_probe) {
        return identity;
    }
    ProcessIdentity {
        pid: lease.runtime.worker_pid,
        start_time: lease.runtime.worker_start_time,
    }
}

#[cfg(test)]
impl SecondaryCustodyLease {
    pub(in super::super) fn probe_worker_identity(&self, identity: ProcessIdentity) {
        *crate::host::lock_recover(&self.worker_identity_probe) = Some(identity);
    }
}
