use super::super::SecondaryCustodyLease;
use crate::process::{BootIdentityRelation, ProcessIdentity, ProcessIdentityObservation};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WorkerCustodyObservation {
    Current,
    Indeterminate,
    ProvenGone,
}

pub(super) async fn observe_worker_after_disconnect(
    lease: &SecondaryCustodyLease,
) -> WorkerCustodyObservation {
    let deadline = tokio::time::Instant::now()
        + crate::session_timing::PROCESS_CAPABILITY_EXIT_EVENT_CONVERGENCE_TIMEOUT;
    loop {
        let observation = observe_worker(lease);
        if observation != WorkerCustodyObservation::Current {
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

pub(in super::super) fn worker_is_proven_gone(lease: &SecondaryCustodyLease) -> bool {
    matches!(observe_worker(lease), WorkerCustodyObservation::ProvenGone)
}

pub(in super::super) async fn worker_is_proven_gone_after_disconnect(
    lease: &SecondaryCustodyLease,
) -> bool {
    matches!(
        observe_worker_after_disconnect(lease).await,
        WorkerCustodyObservation::ProvenGone
    )
}

pub(super) fn observe_worker(lease: &SecondaryCustodyLease) -> WorkerCustodyObservation {
    let process = worker_process(lease).observe();
    match crate::process::boot_identity_relation(&lease.runtime.boot_id, &lease.host_boot_id) {
        BootIdentityRelation::Different => WorkerCustodyObservation::ProvenGone,
        BootIdentityRelation::Same => process_observation(process),
        BootIdentityRelation::Unknown => match process {
            ProcessIdentityObservation::Matching => WorkerCustodyObservation::Current,
            ProcessIdentityObservation::Gone | ProcessIdentityObservation::Unknown => {
                WorkerCustodyObservation::Indeterminate
            }
        },
    }
}

fn process_observation(observation: ProcessIdentityObservation) -> WorkerCustodyObservation {
    match observation {
        ProcessIdentityObservation::Matching => WorkerCustodyObservation::Current,
        ProcessIdentityObservation::Gone => WorkerCustodyObservation::ProvenGone,
        ProcessIdentityObservation::Unknown => WorkerCustodyObservation::Indeterminate,
    }
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
