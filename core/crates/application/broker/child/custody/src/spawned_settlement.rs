use super::failure::{ChildSettlementFailure, OperationFailure};
use anyhow::Result;
use std::io;
use std::process::ExitStatus;
use yolo_process::DetachedChildCustody;

pub(super) const POLL_CHILD_CONTEXT: &str = "poll detached broker child";
const TERMINATE_CHILD_CONTEXT: &str = "terminate detached broker child";

pub(super) fn observe_child(child: &mut DetachedChildCustody) -> Result<Option<ExitStatus>> {
    child.try_wait().map_err(|error| {
        ChildSettlementFailure::one(OperationFailure::wait(POLL_CHILD_CONTEXT, error)).into()
    })
}

pub(super) trait ChildLifecycle {
    fn poll_exited(&mut self) -> io::Result<bool>;
    fn terminate(&mut self) -> io::Result<()>;
}

impl ChildLifecycle for DetachedChildCustody {
    fn poll_exited(&mut self) -> io::Result<bool> {
        self.try_wait().map(|status| status.is_some())
    }

    fn terminate(&mut self) -> io::Result<()> {
        #[cfg(test)]
        if let Some(error) = super::settlement_test_hook::take_termination_failure() {
            return Err(error);
        }
        self.request_direct_termination()
    }
}

enum Settlement {
    Settled(Result<()>),
    Retain {
        failure: Option<ChildSettlementFailure>,
        termination_required: bool,
    },
}

pub fn settle_child(
    mut child: DetachedChildCustody,
    reservation: super::termination::TerminationReservation,
) -> Result<()> {
    match settlement_actions(&mut child) {
        Settlement::Settled(result) => result,
        Settlement::Retain {
            failure,
            termination_required,
        } => retain_unsettled(child, reservation, failure, termination_required),
    }
}

pub fn settle_child_on_drop(
    mut child: DetachedChildCustody,
    reservation: super::termination::TerminationReservation,
) {
    if let Settlement::Retain {
        termination_required,
        ..
    } = settlement_actions(&mut child)
    {
        retain_for_drop(child, reservation, termination_required);
    }
}

#[cfg(test)]
pub(super) fn settle_test_child(child: &mut impl ChildLifecycle) -> Result<()> {
    match settlement_actions(child) {
        Settlement::Settled(result) => result,
        Settlement::Retain {
            failure: Some(failure),
            ..
        } => Err(failure.into()),
        Settlement::Retain { failure: None, .. } => Ok(()),
    }
}

fn settlement_actions(child: &mut impl ChildLifecycle) -> Settlement {
    let mut failures = Vec::new();
    match child.poll_exited() {
        Ok(true) => return Settlement::Settled(Ok(())),
        Ok(false) => {}
        Err(error) => {
            let failure = OperationFailure::wait(POLL_CHILD_CONTEXT, error);
            if failure.wait_authority_lost() {
                return Settlement::Settled(Err(ChildSettlementFailure::one(failure).into()));
            }
            failures.push(failure);
        }
    }
    let termination_required = match child.terminate() {
        Ok(()) => false,
        Err(error) if process_is_gone(&error) => false,
        Err(error) => {
            failures.push(OperationFailure::operation(TERMINATE_CHILD_CONTEXT, error));
            true
        }
    };
    let failure = (!failures.is_empty()).then(|| ChildSettlementFailure::from_failures(failures));
    Settlement::Retain {
        failure,
        termination_required,
    }
}

fn retain_unsettled(
    child: DetachedChildCustody,
    reservation: super::termination::TerminationReservation,
    failure: Option<ChildSettlementFailure>,
    termination_required: bool,
) -> Result<()> {
    reservation.retain(child, termination_required);
    failure.map_or(Ok(()), |failure| Err(failure.into()))
}

fn retain_for_drop(
    child: DetachedChildCustody,
    reservation: super::termination::TerminationReservation,
    termination_required: bool,
) {
    reservation.retain(child, termination_required);
}

fn process_is_gone(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ESRCH)
}
