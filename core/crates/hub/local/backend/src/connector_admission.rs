use crate::LocalBackend;
use yolo_agent_contract::AgentState;
use yolo_hub_connector_registry::RuntimeAdmission;
use yolo_hub_identity::RuntimeTarget;
use yolo_terminal_model::{RecoveryState, TerminalRuntimeStatus, TerminalView};

impl LocalBackend {
    pub(crate) async fn connector_admission(&self, target: RuntimeTarget) -> RuntimeAdmission {
        match self
            .client
            .get_session(target.address.terminal_id.as_uuid())
            .await
        {
            Ok(view) => admission(target, &view),
            Err(_) => RuntimeAdmission::Unavailable,
        }
    }
}

pub(crate) fn native_owned(view: &TerminalView) -> bool {
    view.agent
        .as_ref()
        .is_some_and(|agent| agent.state != AgentState::Exited)
}

pub(crate) fn admission(target: RuntimeTarget, view: &TerminalView) -> RuntimeAdmission {
    if view.manifest.terminal_id != target.address.terminal_id.as_uuid() {
        return RuntimeAdmission::Unavailable;
    }
    if view.manifest.archived {
        return RuntimeAdmission::Retired;
    }
    let Some(runtime) = &view.runtime else {
        return RuntimeAdmission::Unavailable;
    };
    if runtime.runtime_epoch != target.runtime_epoch.as_uuid()
        || matches!(
            runtime.status,
            TerminalRuntimeStatus::Exited | TerminalRuntimeStatus::Lost
        )
    {
        return RuntimeAdmission::Retired;
    }
    if !matches!(
        runtime.status,
        TerminalRuntimeStatus::Running | TerminalRuntimeStatus::OutputBlocked
    ) || view.manifest.recovery_state == RecoveryState::Uncertain
    {
        return RuntimeAdmission::Unavailable;
    }
    if native_owned(view) {
        RuntimeAdmission::NativeOwned
    } else {
        RuntimeAdmission::Available(target)
    }
}
