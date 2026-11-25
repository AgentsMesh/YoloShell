//! Target-scoped agent observation.
//!
//! The classifier validates a terminal view and asks that session's Worker for
//! the current control observation. Callers choose which terminals to observe:
//! control and get observe the target, while a catalogue sweep may observe
//! only changed bindings.

use yolo_agent_control_contract::{NativeControlObservation, binding_digest};
use yolo_runtime_protocol::RuntimeRef;
use yolo_terminal_model::{RecoveryState, TerminalRuntimeStatus, TerminalView};
use yolo_wire::capability;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationStage {
    Coherent,
    RuntimeAbsent,
    BindingAbsent,
    Archived,
    RecoveryUncertain,
    RuntimeNotRunning,
    AgentNotRunning,
    CapabilityUnavailable,
    BindingDigest,
    WorkerDeadline,
    WorkerProtocol,
    WorkerRejected,
    WorkerClient,
    SessionAbsent,
    HostCatalogueUnavailable,
    WorkerObservationAbsent,
    BindingMismatch,
    SessionMismatch,
    RuntimeMismatch,
    WorkerMismatch,
    DigestMismatch,
}

/// Validate the view and read one Worker's current control observation.
pub async fn classify(
    client: &yolo_client::Client,
    view: &TerminalView,
) -> Result<NativeControlObservation, ObservationStage> {
    let runtime = view
        .runtime
        .as_ref()
        .ok_or(ObservationStage::RuntimeAbsent)?;
    let binding = view.agent.as_ref().ok_or(ObservationStage::BindingAbsent)?;
    if view.manifest.archived {
        return Err(ObservationStage::Archived);
    }
    if view.manifest.recovery_state == RecoveryState::Uncertain {
        return Err(ObservationStage::RecoveryUncertain);
    }
    if !matches!(
        runtime.status,
        TerminalRuntimeStatus::Running | TerminalRuntimeStatus::OutputBlocked
    ) {
        return Err(ObservationStage::RuntimeNotRunning);
    }
    if !binding.state.is_active() {
        return Err(ObservationStage::AgentNotRunning);
    }
    if !runtime.capabilities.as_ref().is_some_and(|values| {
        values
            .iter()
            .any(|value| value == capability::NATIVE_AGENT_CONTROL)
    }) {
        return Err(ObservationStage::CapabilityUnavailable);
    }
    let digest = binding_digest(binding).map_err(|_| ObservationStage::BindingDigest)?;
    let observed = client
        .native_agent_observation(
            RuntimeRef::new(view.manifest.terminal_id, runtime.runtime_epoch),
            digest,
        )
        .await
        .map_err(|error| match error {
            yolo_client::ClientError::Deadline => ObservationStage::WorkerDeadline,
            yolo_client::ClientError::Protocol(_) => ObservationStage::WorkerProtocol,
            yolo_client::ClientError::Remote { .. } => ObservationStage::WorkerRejected,
            _ => ObservationStage::WorkerClient,
        })?
        .ok_or(ObservationStage::WorkerObservationAbsent)?;
    let witness = observed.witness().fields();
    if observed.binding() != binding {
        return Err(ObservationStage::BindingMismatch);
    }
    if witness.terminal_id != view.manifest.terminal_id {
        return Err(ObservationStage::SessionMismatch);
    }
    if witness.runtime_epoch != runtime.runtime_epoch {
        return Err(ObservationStage::RuntimeMismatch);
    }
    if witness.worker_id != runtime.worker_id {
        return Err(ObservationStage::WorkerMismatch);
    }
    if witness.binding_digest != digest {
        return Err(ObservationStage::DigestMismatch);
    }
    Ok(observed)
}
