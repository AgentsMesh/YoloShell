use yolo_agent_contract::AgentRecoveryAvailability;
use yolo_hub_terminal_protocol::TerminalRecoveryHint;
use yolo_terminal_model::{BootstrapDelivery, RecoveryState, TerminalRuntimeStatus, TerminalView};
use yolo_wire::capability;

pub fn safe_termination(view: &TerminalView) -> Option<bool> {
    view.runtime.as_ref()?.capabilities.as_ref().map(|values| {
        values
            .iter()
            .any(|value| value == capability::SAFE_PROCESS_TREE_TERMINATION)
    })
}

pub fn recovery_hint(view: &TerminalView) -> TerminalRecoveryHint {
    if view.manifest.archived {
        return TerminalRecoveryHint::Unavailable;
    }
    if view
        .runtime
        .as_ref()
        .is_some_and(|runtime| runtime.status == TerminalRuntimeStatus::Lost)
    {
        return TerminalRecoveryHint::Recreate;
    }
    match view.manifest.recovery_state {
        RecoveryState::Restoring => return TerminalRecoveryHint::Recovering,
        RecoveryState::NeedsRecreate => return TerminalRecoveryHint::Recreate,
        RecoveryState::Uncertain => {
            let survivor = view.runtime.as_ref().is_some_and(|runtime| {
                runtime.status == TerminalRuntimeStatus::Running
                    && (runtime.bootstrap_delivery == BootstrapDelivery::Survivor
                        || safe_termination(view) == Some(false))
            });
            if survivor {
                return TerminalRecoveryHint::Ready;
            }
        }
        _ if view
            .runtime
            .as_ref()
            .is_some_and(|runtime| !matches!(runtime.status, TerminalRuntimeStatus::Exited)) =>
        {
            return TerminalRecoveryHint::Ready;
        }
        _ => {}
    }
    match &view.agent {
        Some(agent) if agent.recovery_availability == AgentRecoveryAvailability::Available => {
            TerminalRecoveryHint::Resume
        }
        Some(_) => TerminalRecoveryHint::Unavailable,
        None => TerminalRecoveryHint::Recreate,
    }
}
