#[path = "pending_lifecycle.rs"]
mod lifecycle;

use anyhow::Result;
use yolo_process::{DetachedChildCustody, handoff_detached_child_custody};
#[path = "pending_supervision.rs"]
mod supervision_constructor;

#[path = "spawned_failure.rs"]
mod failure;
#[path = "spawned_result.rs"]
mod result;
#[path = "spawned_settlement.rs"]
mod settlement;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/spawned_settlement_test_hook.rs"]
mod settlement_test_hook;
#[path = "spawned_state.rs"]
mod state;
#[path = "supervised_broker.rs"]
mod supervised;
#[path = "supervised_containment.rs"]
mod supervised_containment;
#[path = "supervised_wait.rs"]
mod supervised_wait;
#[path = "spawned_termination_reaper.rs"]
mod termination;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/spawned_test_support.rs"]
mod test_support;
#[cfg(test)]
use failure::ChildSettlementFailure;
#[cfg(test)]
use result::combine_cleanup;
pub use result::combine_cleanup_failure;
#[cfg(not(test))]
use result::combine_cleanup_results;
#[cfg(test)]
use settlement::{ChildLifecycle, POLL_CHILD_CONTEXT, settle_test_child};
use settlement::{observe_child, settle_child, settle_child_on_drop};
pub use settlement::{
    settle_child as settle_supervised_child,
    settle_child_on_drop as settle_supervised_child_on_drop,
};
#[cfg(test)]
pub(crate) use settlement_test_hook::install_termination_failure;
use state::DetachedBroker;
pub use state::PendingObservation;
pub(super) use supervised::SupervisedBroker;
pub use termination::{
    TerminationReservation, initialize as initialize_termination_reaper,
    reserve as reserve_termination_custody, wait_for_custody_settlement,
    wait_for_termination_delivery,
};

const HANDOFF_CHILD_CONTEXT: &str = "handoff detached broker child to reaper";

pub struct PendingBroker {
    detached: Option<DetachedBroker>,
    supervised: Option<SupervisedBroker>,
    #[cfg(test)]
    observer: Option<Box<dyn FnMut() -> Result<PendingObservation> + Send>>,
    #[cfg(test)]
    compensation: Option<Box<dyn FnOnce() -> Result<()> + Send>>,
}

impl PendingBroker {
    pub fn compensate(mut self) -> Result<()> {
        let supervised_result = self
            .supervised
            .take()
            .map(SupervisedBroker::compensate)
            .unwrap_or(Ok(()));
        let child_result = self
            .detached
            .take()
            .map(|detached| settle_child(detached.child, detached.termination))
            .unwrap_or(Ok(()));
        #[cfg(test)]
        {
            self.observer.take();
            let callback_result = self
                .compensation
                .take()
                .map(|compensation| compensation())
                .unwrap_or(Ok(()));
            return combine_cleanup(
                combine_cleanup(supervised_result, child_result),
                callback_result,
            );
        }
        #[cfg(not(test))]
        combine_cleanup_results(supervised_result, child_result)
    }

    pub(super) fn handoff(mut self) -> Result<()> {
        if let Some(supervised) = self.supervised.take() {
            return supervised.adopt();
        }
        let Some(detached) = self.detached.take() else {
            #[cfg(test)]
            self.disarm_test_custody();
            return Ok(());
        };
        let Err(failure) = handoff_detached_child_custody(detached.child) else {
            #[cfg(test)]
            self.disarm_test_custody();
            return Ok(());
        };
        let (source, child) = failure.into_parts();
        self.compensate_handoff_failure(source, child, detached.termination)
    }

    pub(super) fn compensate_handoff_failure(
        mut self,
        source: anyhow::Error,
        child: std::process::Child,
        termination: TerminationReservation,
    ) -> Result<()> {
        let child = DetachedChildCustody::capture(child)
            .unwrap_or_else(yolo_process::DetachedChildCustodyError::into_custody);
        self.detached = Some(DetachedBroker { child, termination });
        let primary = source.context(HANDOFF_CHILD_CONTEXT);
        let cleanup = self.compensate();
        Err(combine_cleanup_failure(primary, cleanup))
    }
}

impl Drop for PendingBroker {
    fn drop(&mut self) {
        if let Some(supervised) = self.supervised.take() {
            supervised.abandon();
        }
        if let Some(detached) = self.detached.take() {
            settle_child_on_drop(detached.child, detached.termination);
        }
        #[cfg(test)]
        self.observer.take();
        #[cfg(test)]
        if let Some(compensation) = self.compensation.take() {
            let _ = compensation();
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
include!("../unit_tests/spawned_test_modules.rs");

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/spawned_process.rs"]
mod process_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../test_support/wait.rs"]
mod wait;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/wait_authority.rs"]
mod wait_authority_tests;
