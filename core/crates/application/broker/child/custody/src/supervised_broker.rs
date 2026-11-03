use super::settlement::{settle_child, settle_child_on_drop};
use super::supervised_wait::WaitOutcome;
use super::termination::TerminationReservation;
use crate::pending::combine_cleanup_failure;
use anyhow::{Context, Result};
use std::process::ExitStatus;
use std::time::Instant;
use yolo_broker_supervision_state::SupervisionPaths;
use yolo_broker_supervision_state::{SupervisionPhase, SupervisionRecord};
use yolo_process::{
    DetachedChildCustody, DetachedChildCustodyError, handoff_detached_child_custody,
};
use yolo_remote_broker::BrokerStatus;

const ADOPTION_TIMEOUT: &str = "broker supervisor adoption acknowledgement timed out";
const ADOPTION_EXITED: &str = "broker supervisor exited before adoption acknowledgement";
const CLEANUP_TIMEOUT: &str = "broker supervisor cleanup acknowledgement timed out";
const HANDOFF_CONTEXT: &str = "handoff durable broker supervisor";

pub(crate) struct SupervisedBroker {
    supervisor: Option<DetachedChildCustody>,
    termination: Option<TerminationReservation>,
    expected: SupervisionRecord,
    paths: SupervisionPaths,
    deadline: Instant,
}

impl SupervisedBroker {
    pub(crate) fn new(
        supervisor: DetachedChildCustody,
        termination: TerminationReservation,
        expected: SupervisionRecord,
        paths: SupervisionPaths,
        deadline: Instant,
    ) -> Self {
        Self {
            supervisor: Some(supervisor),
            termination: Some(termination),
            expected,
            paths,
            deadline,
        }
    }

    pub(super) fn observe(&mut self) -> Result<Option<ExitStatus>> {
        self.supervisor
            .as_mut()
            .context(HANDOFF_CONTEXT)?
            .try_wait()
            .map_err(Into::into)
    }

    pub(super) fn matches_status(&self, status: &BrokerStatus) -> bool {
        self.expected.matches_status(status)
    }

    pub(super) fn adopt(mut self) -> Result<()> {
        if let Err(primary) = self.paths.write_adoption(&self.expected) {
            return self.fail_with_direct_settlement(primary);
        }
        let outcome = super::supervised_wait::wait(
            self.supervisor.as_mut().context(HANDOFF_CONTEXT)?,
            &self.paths,
            &self.expected,
            self.deadline,
            |phase| phase == SupervisionPhase::Running,
        );
        match outcome {
            Ok(WaitOutcome::Reached) => {
                let result = self.handoff_supervisor();
                if result.is_ok() {
                    let _ = self.paths.remove_adoption();
                }
                result
            }
            Ok(WaitOutcome::SupervisorExited) => {
                self.rollback_after_adoption(anyhow::anyhow!(ADOPTION_EXITED))
            }
            Ok(WaitOutcome::Deadline) => {
                self.rollback_after_adoption(anyhow::anyhow!(ADOPTION_TIMEOUT))
            }
            Err(primary) => self.rollback_after_adoption(primary),
        }
    }

    pub(super) fn compensate(mut self) -> Result<()> {
        self.terminate_or_contain()
    }

    pub(super) fn abandon(self) {
        let _ = self.compensate();
    }

    fn rollback_after_adoption(mut self, primary: anyhow::Error) -> Result<()> {
        let cleanup = self.terminate_or_contain();
        Err(combine_cleanup_failure(primary, cleanup))
    }

    fn fail_with_direct_settlement(mut self, primary: anyhow::Error) -> Result<()> {
        let cleanup = self.settle_with_broker_containment();
        Err(combine_cleanup_failure(primary, cleanup))
    }

    fn handoff_supervisor(&mut self) -> Result<()> {
        let supervisor = self.supervisor.take().context(HANDOFF_CONTEXT)?;
        match handoff_detached_child_custody(supervisor) {
            Ok(()) => {
                self.termination.take();
                Ok(())
            }
            Err(error) => {
                let (source, child) = error.into_parts();
                let custody = DetachedChildCustody::capture(child)
                    .unwrap_or_else(DetachedChildCustodyError::into_custody);
                self.supervisor = Some(custody);
                Err(combine_cleanup_failure(
                    source.context(HANDOFF_CONTEXT),
                    self.settle_supervisor(),
                ))
            }
        }
    }

    fn settle_supervisor(&mut self) -> Result<()> {
        let child = self.supervisor.take().context(HANDOFF_CONTEXT)?;
        let reservation = self.termination.take().context(HANDOFF_CONTEXT)?;
        settle_child(child, reservation)
    }

    fn settle_with_broker_containment(&mut self) -> Result<()> {
        let broker = super::supervised_containment::contain(&self.paths, &self.expected);
        let supervisor = self.settle_supervisor();
        super::supervised_containment::combine(broker, supervisor)
    }

    fn terminate_or_contain(&mut self) -> Result<()> {
        if let Err(primary) = self.paths.write_termination(&self.expected) {
            return Err(combine_cleanup_failure(
                primary,
                self.settle_with_broker_containment(),
            ));
        }
        let wait = super::supervised_wait::wait(
            self.supervisor.as_mut().context(HANDOFF_CONTEXT)?,
            &self.paths,
            &self.expected,
            self.deadline,
            |phase| phase == SupervisionPhase::Settled,
        );
        match wait {
            Ok(WaitOutcome::Reached) => {
                let handoff = self.handoff_supervisor();
                if handoff.is_ok() {
                    let _ = self.paths.remove_requests();
                }
                handoff
            }
            Ok(WaitOutcome::SupervisorExited) => self.settle_with_broker_containment(),
            Ok(WaitOutcome::Deadline) => Err(combine_cleanup_failure(
                anyhow::anyhow!(CLEANUP_TIMEOUT),
                self.settle_with_broker_containment(),
            )),
            Err(primary) => Err(combine_cleanup_failure(
                primary,
                self.settle_with_broker_containment(),
            )),
        }
    }
}

impl Drop for SupervisedBroker {
    fn drop(&mut self) {
        if let (Some(child), Some(reservation)) = (self.supervisor.take(), self.termination.take())
        {
            settle_child_on_drop(child, reservation);
        }
    }
}
