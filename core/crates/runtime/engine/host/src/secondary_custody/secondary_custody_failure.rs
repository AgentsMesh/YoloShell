use super::*;

#[path = "secondary_custody_worker_observation.rs"]
mod worker_observation;
use worker_observation::{
    WorkerCustodyObservation, observe_worker, observe_worker_after_disconnect,
};
pub(super) use worker_observation::{
    worker_is_proven_gone, worker_is_proven_gone_after_disconnect,
};

const CURRENT_WORKER_FAIL_STOP_CONTEXT: &str =
    "current Worker lost its established PTY custody control stream; Host fail-stop required";
const INDETERMINATE_WORKER_FAIL_STOP_CONTEXT: &str = "Worker identity became indeterminate after PTY custody control failure; Host fail-stop required";
const MONITOR_PANIC_CONTEXT: &str = "PTY custody monitor panicked";
const MISSING_PLATFORM_FAIL_STOP_DETAIL: &str =
    "PTY custody monitor failed after uncommitted platform release";

pub(super) struct CustodyMonitorFault {
    state: SecondaryCustodyState,
    detail: String,
    proven_gone: ProvenGoneDisposition,
    await_worker_exit: bool,
}

#[derive(Clone, Copy)]
enum ProvenGoneDisposition {
    Release,
    Supersede,
}

impl CustodyMonitorFault {
    pub(super) fn disconnected(detail: impl Into<String>) -> Self {
        Self {
            state: SecondaryCustodyState::Disconnected,
            detail: detail.into(),
            proven_gone: ProvenGoneDisposition::Release,
            await_worker_exit: true,
        }
    }

    pub(super) fn failed(detail: impl Into<String>) -> Self {
        Self {
            state: SecondaryCustodyState::Failed,
            detail: detail.into(),
            proven_gone: ProvenGoneDisposition::Release,
            await_worker_exit: false,
        }
    }

    pub(super) fn guardian_gone(detail: impl Into<String>) -> Self {
        let mut fault = Self::failed(detail);
        fault.await_worker_exit = true;
        fault
    }

    pub(super) fn panic(detail: String) -> Self {
        Self::failed(format!("{MONITOR_PANIC_CONTEXT}: {detail}"))
    }

    pub(super) fn for_supersede(mut self) -> Self {
        self.proven_gone = ProvenGoneDisposition::Supersede;
        self
    }
}

pub(super) async fn respond(lease_ref: Weak<SecondaryCustodyLease>, fault: CustodyMonitorFault) {
    let Some(lease) = lease_ref.upgrade() else {
        return;
    };
    if !lease.transition_fault(fault.state) {
        return;
    }
    #[cfg(test)]
    test_hook::pause_after_fault_transition(lease.runtime.terminal_id).await;
    let observation = if fault.await_worker_exit {
        observe_worker_after_disconnect(&lease).await
    } else {
        observe_worker(&lease)
    };
    match observation {
        WorkerCustodyObservation::ProvenGone => {
            if !lease.has_platform() {
                super::containment::fail_stop(
                    &lease.request_drain,
                    MISSING_PLATFORM_FAIL_STOP_DETAIL.to_owned(),
                );
                return;
            }
            drop(lease);
            match fault.proven_gone {
                ProvenGoneDisposition::Release => {
                    super::settlement::retain_until_settled(lease_ref).await
                }
                ProvenGoneDisposition::Supersede => {
                    super::settlement::retain_until_superseded(lease_ref).await
                }
            }
        }
        WorkerCustodyObservation::Current => {
            super::containment::fail_stop(
                &lease.request_drain,
                format!("{CURRENT_WORKER_FAIL_STOP_CONTEXT}: {}", fault.detail),
            );
        }
        WorkerCustodyObservation::Indeterminate => {
            super::containment::fail_stop(
                &lease.request_drain,
                format!("{INDETERMINATE_WORKER_FAIL_STOP_CONTEXT}: {}", fault.detail),
            );
        }
    }
}

pub(super) fn fail_closed(lease_ref: Weak<SecondaryCustodyLease>, detail: String) {
    let Some(lease) = lease_ref.upgrade() else {
        return;
    };
    if !lease.transition_fault(SecondaryCustodyState::Failed) {
        return;
    }
    super::containment::fail_stop(&lease.request_drain, detail);
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/unit/secondary_custody/secondary_custody_failure_test_hook.rs"]
pub(super) mod test_hook;
