use crate::{AgentProjection, constants::*, observation};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_agent_contract::AgentState;
use yolo_agent_control_contract::{
    NativeControlObservation, NativeControlWitness, binding_digest,
};
use yolo_hub_agent_protocol::{
    AgentControlRequest, AgentSnapshot, AgentSnapshotFields, Capability,
    FIRST_OBSERVATION_SEQUENCE, Freshness, Lifecycle, ObservationCursor, ObservationSource,
    Reachability, UnavailabilityReason,
};
use yolo_hub_identity::{AgentTarget, TerminalId};
use yolo_terminal_model::{RecoveryState, TerminalRuntimeStatus, TerminalView};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ControlWitness {
    pub cursor: ObservationCursor,
    pub witness: NativeControlWitness,
}

impl AgentProjection {
    /// Historical witnesses permit receipt lookup only, never a new effect.
    pub fn retained_control_witness(
        &self,
        target: AgentTarget,
    ) -> Result<Option<NativeControlWitness>> {
        self.verify()?;
        ensure!(target.address.hub_id == self.record.hub, WRONG_ORIGIN);
        let current = self
            .record
            .entries
            .get(&target.address.agent_id)
            .filter(|entry| entry.snapshot.cursor().target() == target)
            .and_then(|entry| entry.control_witness.as_ref());
        Ok(current
            .or_else(|| {
                self.record
                    .retired_controls
                    .get(&target.execution_id)
                    .filter(|control| control.cursor.target() == target)
            })
            .map(|entry| entry.witness.clone()))
    }

    /// Exact public cursor/capability selection, still subject to Worker admission.
    pub fn control_witness(
        &self,
        request: &AgentControlRequest,
    ) -> Result<Option<NativeControlWitness>> {
        self.verify()?;
        let target = request.fields().target;
        ensure!(target.address.hub_id == self.record.hub, WRONG_ORIGIN);
        let Some(entry) = self.record.entries.get(&target.address.agent_id) else {
            return Ok(None);
        };
        request.validate_against(&entry.snapshot)?;
        let Some(control) = &entry.control_witness else {
            return Ok(None);
        };
        ensure!(control.cursor == entry.snapshot.cursor(), INVALID_RECORD);
        Ok(Some(control.witness.clone()))
    }

    /// An idle binding unchanged since its last observation needs no Worker
    /// call: the stored witness still covers this execution and the idle
    /// binding state is itself the readiness fact.
    pub fn reusable_idle_observation(
        &self,
        view: &TerminalView,
    ) -> Result<Option<NativeControlObservation>> {
        self.verify()?;
        if view.manifest.archived || view.manifest.recovery_state == RecoveryState::Uncertain {
            return Ok(None);
        }
        let Some(binding) = view
            .agent
            .as_ref()
            .filter(|binding| binding.state == AgentState::WaitingForInput)
        else {
            return Ok(None);
        };
        let Some(runtime) = view.runtime.as_ref().filter(|runtime| {
            matches!(
                runtime.status,
                TerminalRuntimeStatus::Running | TerminalRuntimeStatus::OutputBlocked
            )
        }) else {
            return Ok(None);
        };
        let session = TerminalId::from_uuid(view.manifest.terminal_id)?;
        let entry = self.record.latest.get(&session).and_then(|id| self.record.entries.get(id));
        let Some(entry) = entry else { return Ok(None) };
        // An idle binding can still have a pending interaction; reusing the
        // stored witness would silently drop it.
        if entry.snapshot.fields().interaction.is_some() {
            return Ok(None);
        }
        let Some(control) = entry.control_witness.as_ref() else {
            return Ok(None);
        };
        let fields = control.witness.fields();
        if fields.terminal_id != view.manifest.terminal_id
            || fields.runtime_epoch != runtime.runtime_epoch
            || fields.worker_id != runtime.worker_id
            || fields.binding_digest != binding_digest(binding)?
        {
            return Ok(None);
        }
        Ok(NativeControlObservation::new(binding.clone(), control.witness.clone(), true).ok())
    }
}

pub(crate) fn select(
    view: &TerminalView,
    observed: Option<&NativeControlObservation>,
    previous: Option<&ControlWitness>,
) -> Result<(Option<NativeControlWitness>, bool)> {
    let Some(observed) = observed else {
        return Ok((previous.map(|entry| entry.witness.clone()), false));
    };
    let fields = observed.witness().fields();
    ensure!(
        view.agent.as_ref() == Some(observed.binding())
            && fields.terminal_id == view.manifest.terminal_id
            && view
                .runtime
                .as_ref()
                .is_some_and(|runtime| fields.runtime_epoch == runtime.runtime_epoch
                    && fields.worker_id == runtime.worker_id),
        INVALID_BINDING
    );
    Ok((Some(observed.witness().clone()), true))
}

pub(crate) fn capability(
    fields: &mut AgentSnapshotFields,
    retained: bool,
    coherent: bool,
    prompt_input_ready: bool,
) {
    fields.capabilities.interrupt = if coherent
        && fields.source == ObservationSource::ProviderHook
        && fields.freshness == Freshness::Current
        && fields.reachability == Reachability::Reachable
        && matches!(fields.lifecycle, Lifecycle::Running {})
    {
        Capability::Available {}
    } else if retained {
        Capability::Unavailable {
            reason: UnavailabilityReason::StateUnknown,
        }
    } else {
        Capability::Unsupported {}
    };
    fields.capabilities.submit_prompt = match fields.capabilities.interrupt {
        Capability::Available {}
            if prompt_input_ready
                && fields.activity == yolo_hub_agent_protocol::Activity::WaitingForInput =>
        {
            Capability::Available {}
        }
        Capability::Available {} => Capability::Unavailable {
            reason: UnavailabilityReason::InputUnavailable,
        },
        capability => capability,
    };
}


pub(crate) fn revise(
    previous: &AgentSnapshot,
    mut fields: AgentSnapshotFields,
    source_changed: bool,
) -> Result<AgentSnapshot> {
    if source_changed
        && previous.fields() == &fields
        && previous.cursor().target() == fields.cursor.target()
    {
        fields.cursor = ObservationCursor::new(
            fields.cursor.target(),
            fields.cursor.stream(),
            fields
                .cursor
                .sequence()
                .checked_add(FIRST_OBSERVATION_SEQUENCE)
                .context(CURSOR_EXHAUSTED)?,
        )?;
        return Ok(AgentSnapshot::new(fields)?);
    }
    observation::revise(previous, fields)
}
