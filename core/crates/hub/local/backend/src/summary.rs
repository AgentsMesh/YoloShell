use yolo_hub_identity::{HubId, RuntimeEpoch, RuntimeTarget, TerminalAddress, TerminalId};
use yolo_hub_local_platform::{recovery_hint, safe_termination};
use yolo_hub_terminal_protocol::{
    PublicError, RuntimeStatus, RuntimeSummary, TerminalSummary, TerminalSummaryFields,
};
use yolo_terminal_model::{TerminalRuntimeStatus, TerminalView};

pub(crate) fn summary(hub: HubId, view: &TerminalView) -> Result<TerminalSummary, PublicError> {
    let address = TerminalAddress {
        hub_id: hub,
        terminal_id: TerminalId::from_uuid(view.manifest.terminal_id)
            .map_err(|_| PublicError::Internal)?,
    };
    let runtime = view
        .runtime
        .as_ref()
        .map(|runtime| {
            Ok::<_, PublicError>(RuntimeSummary {
                target: RuntimeTarget {
                    address,
                    runtime_epoch: RuntimeEpoch::from_uuid(runtime.runtime_epoch)
                        .map_err(|_| PublicError::Internal)?,
                },
                status: status(runtime.status),
                output_sequence: runtime.last_output_sequence,
                size: None,
                safe_termination: safe_termination(view),
            })
        })
        .transpose()?;
    TerminalSummary::new(TerminalSummaryFields {
        address,
        name: view.manifest.name.clone(),
        presentation: view
            .runtime
            .as_ref()
            .map(|runtime| runtime.presentation.clone())
            .unwrap_or_default(),
        cwd: view
            .manifest
            .cwd
            .to_str()
            .ok_or(PublicError::InvalidMetadata)?
            .to_owned(),
        archived: view.manifest.archived,
        runtime,
        agent: None,
        recovery: Some(recovery_hint(view)),
        workspace: None,
    })
}

fn status(status: TerminalRuntimeStatus) -> RuntimeStatus {
    match status {
        TerminalRuntimeStatus::Starting => RuntimeStatus::Starting,
        TerminalRuntimeStatus::Running => RuntimeStatus::Running,
        TerminalRuntimeStatus::Finalizing => RuntimeStatus::Finalizing,
        TerminalRuntimeStatus::OutputBlocked => RuntimeStatus::OutputBlocked,
        TerminalRuntimeStatus::Exited => RuntimeStatus::Exited,
        TerminalRuntimeStatus::Lost => RuntimeStatus::Lost,
        TerminalRuntimeStatus::Unreachable => RuntimeStatus::Unreachable,
    }
}
