use super::super::startup_untracked_reaper::StartupUntrackedReaperReservation;
#[cfg(test)]
use super::ReaderBackedPtyOutput;
use super::{ExclusivePtyOutput, PtyChild, StartupChildGuard, StartupOutputProgress};
use crate::process::DirectChildIdentity;
use std::panic::{AssertUnwindSafe, catch_unwind};

impl StartupChildGuard {
    #[cfg(test)]
    pub(crate) fn new(child: PtyChild) -> Self {
        let reservation = StartupUntrackedReaperReservation::reserve_unscoped().unwrap();
        let fault_reaper =
            super::super::pty_fault_reaper::FaultReaperReservation::reserve().unwrap();
        Self::prepared(
            child,
            reservation,
            fault_reaper,
            None,
            Some(super::super::UnreleasedBootstrapGate),
        )
    }

    #[cfg(test)]
    pub(crate) fn with_output(child: PtyChild, output_progress: ExclusivePtyOutput) -> Self {
        let reservation = StartupUntrackedReaperReservation::reserve_unscoped().unwrap();
        let fault_reaper =
            super::super::pty_fault_reaper::FaultReaperReservation::reserve().unwrap();
        Self::prepared(
            child,
            reservation,
            fault_reaper,
            Some(StartupOutputProgress::Exclusive(output_progress)),
            Some(super::super::UnreleasedBootstrapGate),
        )
    }

    pub(crate) fn with_reserved_output(
        child: PtyChild,
        identity: DirectChildIdentity,
        reservation: StartupUntrackedReaperReservation,
        fault_reaper: super::super::pty_fault_reaper::FaultReaperReservation,
        output_progress: ExclusivePtyOutput,
        gate: super::super::UnreleasedBootstrapGate,
    ) -> Self {
        Self::prepared_with_identity(
            child,
            Some(identity),
            reservation,
            fault_reaper,
            Some(StartupOutputProgress::Exclusive(output_progress)),
            Some(gate),
        )
    }

    #[cfg(test)]
    pub(crate) fn with_reader_backed_output(
        child: PtyChild,
        output_progress: ReaderBackedPtyOutput,
    ) -> Self {
        let reservation = StartupUntrackedReaperReservation::reserve_unscoped().unwrap();
        let fault_reaper =
            super::super::pty_fault_reaper::FaultReaperReservation::reserve().unwrap();
        Self::prepared(
            child,
            reservation,
            fault_reaper,
            Some(StartupOutputProgress::ReaderBacked(output_progress)),
            Some(super::super::UnreleasedBootstrapGate),
        )
    }

    pub(crate) fn prepared(
        child: PtyChild,
        reservation: StartupUntrackedReaperReservation,
        fault_reaper: super::super::pty_fault_reaper::FaultReaperReservation,
        output_progress: Option<StartupOutputProgress>,
        unreleased_gate: Option<super::super::UnreleasedBootstrapGate>,
    ) -> Self {
        let identity = contain_capture(|| DirectChildIdentity::capture_candidate(&child));
        Self::prepared_with_identity(
            child,
            identity,
            reservation,
            fault_reaper,
            output_progress,
            unreleased_gate,
        )
    }

    pub(crate) fn prepared_with_identity(
        child: PtyChild,
        identity: Option<DirectChildIdentity>,
        reservation: StartupUntrackedReaperReservation,
        fault_reaper: super::super::pty_fault_reaper::FaultReaperReservation,
        output_progress: Option<StartupOutputProgress>,
        unreleased_gate: Option<super::super::UnreleasedBootstrapGate>,
    ) -> Self {
        let process_id = identity.as_ref().map(|identity| identity.root().pid);
        let process_identity = identity.as_ref().map(DirectChildIdentity::root);
        let live_identity = identity.as_ref().map(DirectChildIdentity::live_root);
        Self {
            child: Some(child),
            authority: None,
            process_id,
            identity: process_identity,
            live_identity,
            tree: None,
            monitor: None,
            output_progress,
            untracked_reaper: Some(reservation),
            fault_reaper: Some(fault_reaper),
            unreleased_gate,
            untracked_gone_committed: false,
        }
    }
}

pub(crate) fn contain_capture<Value>(
    capture: impl FnOnce() -> anyhow::Result<Value>,
) -> Option<Value> {
    match catch_unwind(AssertUnwindSafe(capture)) {
        Ok(Ok(value)) => Some(value),
        Ok(Err(error)) => {
            super::super::terminal_reporting::discard_error(error);
            None
        }
        Err(payload) => {
            let _ = super::super::supervision::panic_detail(payload);
            None
        }
    }
}
