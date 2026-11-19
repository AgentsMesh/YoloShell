use super::{
    FaultOutput, FaultReap, FaultReapCompletion, FaultReapStateObserver, OwnershipFaultHandoff,
    PtyForceTermination, PtyProcessCustody, RootWaitAuthority,
};
use crate::worker::pty_ownership::{PtyCustodyAccess, PtyOwnershipCustody};

pub(crate) fn fault_reap(
    authority: RootWaitAuthority,
    mut processes: PtyProcessCustody,
    force: PtyForceTermination,
    output_progress: FaultOutput,
    missing_eof: crate::worker::pty_missing_eof::MissingEofDeadline,
    root_exit_proof: Option<crate::process::UnreapedRootExit>,
    failure: Option<anyhow::Error>,
    completion: FaultReapCompletion,
    state_observer: FaultReapStateObserver,
) -> FaultReap {
    prepare_processes(&mut processes);
    let custody = PtyOwnershipCustody {
        authority: Some(authority),
        processes: Some(processes),
        force,
        output_progress,
        missing_eof,
        root_exit_proof,
        access: PtyCustodyAccess::Control,
    };
    prepared_fault_reap(
        OwnershipFaultHandoff::new(custody, completion),
        failure,
        state_observer,
        super::schedule::FaultReapSchedule::production(),
    )
}

pub(crate) fn prepare_processes(processes: &mut PtyProcessCustody) {
    let _ = crate::worker::supervision::contain(|| processes.clear_stale_progress());
}

pub(crate) fn prepared_fault_reap(
    handoff: OwnershipFaultHandoff,
    failure: Option<anyhow::Error>,
    state_observer: FaultReapStateObserver,
    schedule: super::schedule::FaultReapSchedule,
) -> FaultReap {
    FaultReap {
        handoff: Some(handoff),
        lost_wait_custody: None,
        failure,
        root_gone_without_wait: false,
        schedule,
        state_observer,
    }
}

#[cfg(test)]
pub(crate) fn passive_fault_reap_for_test() -> FaultReap {
    let custody = PtyOwnershipCustody {
        authority: None,
        processes: None,
        force: PtyForceTermination::new(),
        output_progress: FaultOutput::Reader(
            crate::worker::pty_output_progress::ReaderBackedPtyOutput::for_test(),
        ),
        missing_eof: crate::worker::pty_missing_eof::MissingEofDeadline::production(),
        root_exit_proof: None,
        access: PtyCustodyAccess::Control,
    };
    let mut reaper = prepared_fault_reap(
        OwnershipFaultHandoff::new(custody, FaultReapCompletion::log()),
        None,
        FaultReapStateObserver::default(),
        super::schedule::FaultReapSchedule::production(),
    );
    reaper.enter_observer_only();
    reaper
}

pub(crate) fn vacant_fault_reap(
    state_observer: FaultReapStateObserver,
    schedule: super::schedule::FaultReapSchedule,
) -> FaultReap {
    FaultReap {
        handoff: None,
        lost_wait_custody: None,
        failure: None,
        root_gone_without_wait: false,
        schedule,
        state_observer,
    }
}
