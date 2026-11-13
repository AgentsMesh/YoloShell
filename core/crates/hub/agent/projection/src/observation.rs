use crate::constants::*;
use anyhow::{Context, Result};
use yolo_agent_contract::{AgentBinding, AgentRecoveryAvailability, AgentState};
use yolo_agent_control_contract::{NativeControlObservation, NativeInteractionKind};
use yolo_agent_provider_contract::ProjectionPolicy;
use yolo_hub_agent_protocol::{
    Activity, AgentCapabilities, AgentSnapshot, AgentSnapshotFields, Capability,
    FIRST_OBSERVATION_SEQUENCE, Freshness, Interaction, InteractionChoice, InteractionKind,
    InteractionReference, Lifecycle, ObservationCursor, ObservationSource, Reachability,
    RecoveryStatus,
};
use yolo_terminal_model::{RecoveryState, TerminalRuntimeStatus, TerminalView};

pub(crate) fn normalize(
    fields: &mut AgentSnapshotFields,
    binding: &AgentBinding,
    view: &TerminalView,
    prompt_ready: bool,
    policy: ProjectionPolicy,
) {
    let native_hooks = policy.hook_authoritative
        && binding.recovery_availability == AgentRecoveryAvailability::Available;
    let runtime_matches = view
        .runtime
        .as_ref()
        .is_some_and(|runtime| runtime.runtime_epoch == fields.terminal.runtime_epoch.as_uuid());
    let active_runtime = runtime_matches
        && view.runtime.as_ref().is_some_and(|runtime| {
            matches!(
                runtime.status,
                TerminalRuntimeStatus::Running | TerminalRuntimeStatus::OutputBlocked
            )
        });
    let reachable = runtime_matches
        && view.runtime.as_ref().is_some_and(|runtime| {
            !matches!(
                runtime.status,
                TerminalRuntimeStatus::Unreachable | TerminalRuntimeStatus::Lost
            )
        });
    let state_current = active_runtime && view.manifest.recovery_state != RecoveryState::Uncertain;
    let exited = binding.state == AgentState::Exited;
    fields.lifecycle = if exited {
        Lifecycle::Exited { exit_code: None }
    } else if state_current && binding.state != AgentState::Unknown {
        Lifecycle::Running {}
    } else {
        Lifecycle::Unknown {}
    };
    fields.activity = if exited {
        Activity::Idle
    } else if state_current && native_hooks && prompt_ready {
        // A verified provider turn-ending record resolves a Stop candidate (or
        // an interrupted turn) to the same surface state as an idle PTY.
        Activity::WaitingForInput
    } else if state_current && native_hooks {
        match binding.state {
            AgentState::Running => Activity::Working,
            AgentState::WaitingForInput => Activity::WaitingForInput,
            AgentState::WaitingForPermission => Activity::WaitingForApproval,
            AgentState::Active | AgentState::Unknown | AgentState::Exited => Activity::Unknown,
        }
    } else {
        Activity::Unknown
    };
    fields.reachability = if reachable {
        Reachability::Reachable
    } else {
        Reachability::Unreachable
    };
    fields.freshness = if (state_current && binding.state != AgentState::Unknown) || exited {
        Freshness::Current
    } else {
        Freshness::Stale
    };
    fields.source = if native_hooks {
        ObservationSource::ProviderHook
    } else {
        ObservationSource::ProcessObserver
    };
    fields.observed_unix_ms = binding.updated_at_ms as u64;
    fields.recovery = match binding.recovery_availability {
        AgentRecoveryAvailability::Available
            if view.manifest.recovery_state != RecoveryState::Uncertain =>
        {
            RecoveryStatus::Available
        }
        AgentRecoveryAvailability::Available
        | AgentRecoveryAvailability::Unavailable
        | AgentRecoveryAvailability::UnmanagedLaunch => RecoveryStatus::Unavailable,
    };
    fields.capabilities = AgentCapabilities::UNSUPPORTED;
    fields.interaction = None;
}

/// A pending interaction is answerable only under the same coherent provider
/// observation that enables interrupt: current, reachable and running.
pub(crate) fn apply_interaction(
    fields: &mut AgentSnapshotFields,
    observation: Option<&NativeControlObservation>,
    coherent: bool,
) -> Result<()> {
    let Some(pending) = observation.and_then(NativeControlObservation::interaction) else {
        return Ok(());
    };
    if !coherent
        || fields.source != ObservationSource::ProviderHook
        || fields.freshness != Freshness::Current
        || fields.reachability != Reachability::Reachable
        || !matches!(fields.lifecycle, Lifecycle::Running {})
    {
        return Ok(());
    }
    let kind = match pending.kind() {
        NativeInteractionKind::Approval => InteractionKind::Approval,
        NativeInteractionKind::Choice => InteractionKind::Choice,
        NativeInteractionKind::Text => InteractionKind::Text,
    };
    let choices = pending
        .choices()
        .iter()
        .map(|choice| InteractionChoice {
            id: choice.id.clone(),
            label: choice.label.clone(),
        })
        .collect();
    fields.interaction = Some(
        Interaction::with_prompt(
            InteractionReference {
                target: fields.cursor.target(),
                id: pending.id(),
            },
            kind,
            choices,
            pending.prompt().to_owned(),
        )
        .context("pending interaction is invalid")?,
    );
    fields.capabilities.respond_interaction = Capability::Available {};
    fields.activity = match kind {
        InteractionKind::Approval => Activity::WaitingForApproval,
        InteractionKind::Choice | InteractionKind::Text => Activity::WaitingForInput,
    };
    Ok(())
}

pub(crate) fn revise(
    previous: &AgentSnapshot,
    mut fields: AgentSnapshotFields,
) -> Result<AgentSnapshot> {
    if previous.fields() != &fields && previous.cursor().target() == fields.cursor.target() {
        fields.cursor = ObservationCursor::new(
            fields.cursor.target(),
            fields.cursor.stream(),
            previous
                .cursor()
                .sequence()
                .checked_add(FIRST_OBSERVATION_SEQUENCE)
                .context(CURSOR_EXHAUSTED)?,
        )?;
    }
    Ok(AgentSnapshot::new(fields)?)
}
