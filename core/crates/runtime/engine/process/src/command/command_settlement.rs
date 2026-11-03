use super::child_reaper::{DetachedChildReservation, TerminationPolicy};
use super::is_child_already_reaped;
use anyhow::Result;
use std::fmt;
use std::io;
use std::process::{Child, ExitStatus};

pub(super) const PRIVATE_INPUT_KILL_CONTEXT: &str =
    "request detached process termination after private input failure";
pub(super) const PRIVATE_INPUT_POLL_CONTEXT: &str =
    "poll detached process after private input failure";
const PRIVATE_INPUT_CUSTODY_CONTEXT: &str = "capture detached process cleanup custody";
const FAILURE_SEPARATOR: &str = "; ";

#[derive(Debug)]
pub(super) struct PrivateInputCleanupFailure {
    input: anyhow::Error,
    settlement: anyhow::Error,
}

pub(super) fn combine_private_input_failure(
    input: anyhow::Error,
    settlement: Result<()>,
) -> anyhow::Error {
    match settlement {
        Ok(()) => input,
        Err(settlement) => PrivateInputCleanupFailure { input, settlement }.into(),
    }
}

impl PrivateInputCleanupFailure {
    #[cfg(test)]
    pub(super) fn input(&self) -> &anyhow::Error {
        &self.input
    }

    #[cfg(test)]
    pub(super) fn settlement(&self) -> &anyhow::Error {
        &self.settlement
    }
}

impl fmt::Display for PrivateInputCleanupFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}; {}: {:#}",
            self.input,
            super::command_private_spawn::PRIVATE_INPUT_SETTLEMENT_CONTEXT,
            self.settlement,
        )
    }
}

impl std::error::Error for PrivateInputCleanupFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.input.as_ref())
    }
}

pub(super) fn terminate_after_private_input_failure(
    child: Child,
    reservation: DetachedChildReservation,
) -> Result<()> {
    let mut child = match super::DetachedChildCustody::capture(child) {
        Ok(child) => child,
        Err(error) => {
            tracing::warn!(%error, message = PRIVATE_INPUT_CUSTODY_CONTEXT);
            return terminate_without_exact_custody(error.into_custody(), reservation);
        }
    };
    let mut failures = Vec::new();
    match poll_child(&mut child) {
        Ok(Some(_)) => return Ok(()),
        Ok(None) => {}
        Err(error) if is_child_already_reaped(&error) => return Ok(()),
        Err(error) => failures.push(SettlementOperation::new(PRIVATE_INPUT_POLL_CONTEXT, error)),
    }
    let policy = match request_termination(&mut child) {
        Ok(()) => TerminationPolicy::NaturalExit,
        Err(error) if process_is_gone(&error) => TerminationPolicy::NaturalExit,
        Err(error) => {
            failures.push(SettlementOperation::new(PRIVATE_INPUT_KILL_CONTEXT, error));
            TerminationPolicy::Terminate
        }
    };
    reservation.retain(child, policy);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(SettlementFailures(failures).into())
    }
}

fn terminate_without_exact_custody(
    mut child: super::DetachedChildCustody,
    reservation: DetachedChildReservation,
) -> Result<()> {
    let mut failures = Vec::new();
    match child.try_wait() {
        Ok(Some(_)) => return Ok(()),
        Ok(None) => {}
        Err(error) if is_child_already_reaped(&error) => return Ok(()),
        Err(error) => failures.push(SettlementOperation::new(PRIVATE_INPUT_POLL_CONTEXT, error)),
    }
    let policy = match child.request_direct_termination() {
        Ok(()) => TerminationPolicy::NaturalExit,
        Err(error) if process_is_gone(&error) => TerminationPolicy::NaturalExit,
        Err(error) => {
            failures.push(SettlementOperation::new(PRIVATE_INPUT_KILL_CONTEXT, error));
            TerminationPolicy::Terminate
        }
    };
    reservation.retain(child, policy);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(SettlementFailures(failures).into())
    }
}

pub fn terminate_detached_child(child: Child) -> Result<()> {
    let reservation = super::child_reaper::prepare_detached_child_handoff()?;
    terminate_after_private_input_failure(child, reservation)
}

#[cfg(test)]
pub(super) fn terminate_test_child(child: Child) -> Result<()> {
    let reservation = super::child_reaper::prepare_detached_child_handoff()?;
    terminate_after_private_input_failure(child, reservation)
}

fn poll_child(child: &mut super::DetachedChildCustody) -> io::Result<Option<ExitStatus>> {
    #[cfg(test)]
    if let Some(result) = super::command_settlement_test_hook::poll_override() {
        return result;
    }
    child.try_wait()
}

fn request_termination(child: &mut super::DetachedChildCustody) -> io::Result<()> {
    #[cfg(test)]
    if let Some(error) = super::command_settlement_test_hook::termination_failure() {
        return Err(error);
    }
    child.request_direct_termination()
}

fn process_is_gone(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ESRCH)
}

#[derive(Debug)]
struct SettlementOperation {
    context: &'static str,
    source: io::Error,
}

impl SettlementOperation {
    fn new(context: &'static str, source: io::Error) -> Self {
        Self { context, source }
    }
}

#[derive(Debug)]
pub(super) struct SettlementFailures(Vec<SettlementOperation>);

impl fmt::Display for SettlementFailures {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, failure) in self.0.iter().enumerate() {
            if index > usize::default() {
                formatter.write_str(FAILURE_SEPARATOR)?;
            }
            write!(formatter, "{}: {}", failure.context, failure.source)?;
        }
        Ok(())
    }
}

impl std::error::Error for SettlementFailures {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0
            .first()
            .map(|failure| &failure.source as &(dyn std::error::Error + 'static))
    }
}
