use anyhow::{Context, Result};
use yolo_broker_child_custody::{
    TerminationReservation, settle_supervised_child, settle_supervised_child_on_drop,
};
use yolo_process::DetachedChildCustody;

const CHILD_CUSTODY_MISSING: &str = "supervised broker child custody is missing";
const TERMINATION_CUSTODY_MISSING: &str = "supervised broker termination custody is missing";

pub(super) struct SupervisorCustody {
    child: Option<DetachedChildCustody>,
    termination: Option<TerminationReservation>,
}

impl SupervisorCustody {
    pub(super) fn new(child: DetachedChildCustody, termination: TerminationReservation) -> Self {
        Self {
            child: Some(child),
            termination: Some(termination),
        }
    }

    pub(super) fn child(&mut self) -> Result<&mut DetachedChildCustody> {
        self.child.as_mut().context(CHILD_CUSTODY_MISSING)
    }

    pub(super) fn finish_reaped(mut self) {
        self.child.take();
        self.termination.take();
    }

    pub(super) fn settle(mut self) -> Result<()> {
        let child = self.child.take().context(CHILD_CUSTODY_MISSING)?;
        let termination = self
            .termination
            .take()
            .context(TERMINATION_CUSTODY_MISSING)?;
        settle_supervised_child(child, termination)
    }
}

impl Drop for SupervisorCustody {
    fn drop(&mut self) {
        if let (Some(child), Some(termination)) = (self.child.take(), self.termination.take()) {
            settle_supervised_child_on_drop(child, termination);
        }
    }
}
