use super::super::{FaultOutput, FaultReap};
use crate::worker::pty_process_custody::PtyProcessCustody;

#[derive(Clone, Copy)]
pub(crate) enum InitialRootSignal {
    Graceful,
    Force,
}

pub(crate) fn request(ownership: &mut FaultReap) -> bool {
    let Some(signal) = select(ownership) else {
        return false;
    };
    let custody = ownership.custody_mut();
    let Some(authority) = custody.authority.as_ref() else {
        return false;
    };
    // Exclusive output must cross the frozen output fence before the last slave closes.
    let defer_graceful = match &custody.output_progress {
        FaultOutput::Exclusive(_) => match custody.processes.as_ref() {
            // Explicit failure retires separately, before the force driver runs.
            Some(PtyProcessCustody::Tracking(tree))
                if tree.explicit_bootstrap_exec_failure_is_pending() =>
            {
                false
            }
            Some(processes) => !processes.requires_exact_root_force_fallback(),
            None => false,
        },
        FaultOutput::Reader(_) => false,
    };
    let result = dispatch(
        signal,
        || match &mut custody.output_progress {
            FaultOutput::Reader(output) => output.recovery().progress().map(|_| ()),
            FaultOutput::Exclusive(output) => output.progress().map(|_| ()),
        },
        || match signal {
            InitialRootSignal::Graceful if defer_graceful => Ok(false),
            InitialRootSignal::Graceful => authority.request_graceful_termination(),
            InitialRootSignal::Force => authority.request_force_termination(),
        },
    );
    match result {
        Ok(sent) => sent,
        Err(error) => {
            ownership.remember(Err(error));
            false
        }
    }
}

#[cfg(all(test, feature = "yolo-integration-tests", target_os = "macos"))]
#[path = "../../../../integration_tests/initial_exclusive_output_signal_tests.rs"]
mod tests;

pub(crate) fn dispatch(
    signal: InitialRootSignal,
    resume_output: impl FnOnce() -> anyhow::Result<()>,
    send_signal: impl FnOnce() -> anyhow::Result<bool>,
) -> anyhow::Result<bool> {
    if matches!(signal, InitialRootSignal::Graceful) {
        resume_output()?;
    }
    send_signal()
}

pub(crate) fn select(ownership: &FaultReap) -> Option<InitialRootSignal> {
    if ownership.custody().force.force_was_requested() {
        return requires_exact_root_force_fallback(ownership).then_some(InitialRootSignal::Force);
    }
    ownership
        .custody()
        .force
        .allows_graceful_fault_handoff()
        .then_some(InitialRootSignal::Graceful)
}

pub(crate) fn requires_exact_root_force_fallback(ownership: &FaultReap) -> bool {
    ownership
        .custody()
        .processes
        .as_ref()
        .is_some_and(PtyProcessCustody::requires_exact_root_force_fallback)
}
