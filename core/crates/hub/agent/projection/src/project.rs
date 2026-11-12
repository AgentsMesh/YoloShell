use crate::{AgentProjection, constants::*, identity, observation, record::Entry};
use anyhow::{Result, ensure};
use uuid::Uuid;
use yolo_agent_adapter_contract::{DetectionContext, validation::validate_core};
use yolo_agent_contract::AgentState;
use yolo_agent_control_contract::NativeControlObservation;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, Activity, AgentCapabilities, AgentSnapshot, AgentSnapshotFields,
    AgentWorkingDirectory, FIRST_OBSERVATION_SEQUENCE, Freshness, Lifecycle, ObservationCursor,
    ObservationSource, ObservedProvider, Reachability, RecoveryStatus,
};
use yolo_hub_identity::{
    AgentAddress, AgentId, AgentTarget, ExecutionId, RuntimeEpoch, RuntimeTarget, TerminalAddress,
    TerminalId,
};
use yolo_terminal_model::{TerminalRuntimeStatus, TerminalView};

impl AgentProjection {
    /// Feed serialized, authenticated Host observations. This API must not be an RPC write method.
    pub fn project(&mut self, view: &TerminalView) -> Result<Option<AgentSnapshot>> {
        self.project_with_observation(view, None)
    }

    pub fn project_with_observation(
        &mut self,
        view: &TerminalView,
        observation: Option<&NativeControlObservation>,
    ) -> Result<Option<AgentSnapshot>> {
        self.verify()?;
        let session = TerminalId::from_uuid(view.manifest.terminal_id)?;
        // Archived sessions are history: the live-only sweep would revoke any
        // observation made here, so settle them unreachable like no binding.
        let Some(binding) = view.agent.as_ref().filter(|_| !view.manifest.archived) else {
            let mut next = self.record.clone();
            next.invalidate(Some(session))?;
            self.commit(next)?;
            return Ok(None);
        };
        validate_core(
            binding,
            &DetectionContext::new(view.manifest.terminal_id, view.manifest.cwd.clone(), []),
        )?;
        let subject = identity::subject(binding)?;
        let provider = ObservedProvider::new(
            subject.kind.clone(),
            None,
            Some(subject.native_reference.clone()),
        )?
        .with_display_name(
            self.policies
                .display_name(&binding.kind)
                .map(str::to_owned),
        )?;
        let existing = self
            .record
            .latest
            .get(&session)
            .and_then(|id| self.record.entries.get(id)).filter(|entry| entry.subject == subject);
        let active = binding.state.is_active();
        let observed_runtime = view.runtime.as_ref().filter(|runtime| {
            active
                && matches!(
                    runtime.status,
                    TerminalRuntimeStatus::Running | TerminalRuntimeStatus::OutputBlocked
                )
        });
        let runtime = match observed_runtime {
            Some(runtime) => RuntimeEpoch::from_uuid(runtime.runtime_epoch)?,
            None => match existing {
                Some(entry) => entry.runtime,
                // A historical binding does not identify which terminal execution produced it.
                None => return Ok(None),
            },
        };
        let selected = identity::select(&self.record, &subject, runtime, binding, view)?;
        let previous = selected.and_then(|id| self.record.entries.get(&id));
        let id = selected.unwrap_or_else(AgentId::new);
        let same_execution =
            previous.is_some_and(|entry| identity::same_execution(entry, runtime, binding));
        let target = AgentTarget {
            address: AgentAddress {
                hub_id: self.record.hub,
                agent_id: id,
            },
            execution_id: previous
                .filter(|_| same_execution)
                .map_or_else(ExecutionId::new, |entry| {
                    entry.snapshot.cursor().target().execution_id
                }),
        };
        let cursor = previous.filter(|_| same_execution).map_or_else(
            || ObservationCursor::new(target, Uuid::new_v4(), FIRST_OBSERVATION_SEQUENCE),
            |entry| Ok(entry.snapshot.cursor()),
        )?;
        let mut fields = AgentSnapshotFields {
            schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
            cursor,
            terminal: RuntimeTarget {
                address: TerminalAddress {
                    hub_id: self.record.hub,
                    terminal_id: session,
                },
                runtime_epoch: runtime,
            },
            provider,
            working_directory: Some(AgentWorkingDirectory::new(binding.cwd.clone())?),
            lifecycle: Lifecycle::Unknown {},
            activity: Activity::Unknown,
            reachability: Reachability::Unreachable,
            freshness: Freshness::Stale,
            source: ObservationSource::ProcessObserver,
            observed_unix_ms: binding.updated_at_ms as u64,
            recovery: RecoveryStatus::Unknown,
            capabilities: AgentCapabilities::UNSUPPORTED,
            interaction: None,
        };
        let prompt_ready = observation.is_some_and(NativeControlObservation::prompt_input_ready);
        observation::normalize(
            &mut fields,
            binding,
            view,
            prompt_ready,
            self.policies.policy(&binding.kind),
        );
        let retained = previous
            .filter(|_| same_execution)
            .and_then(|entry| entry.control_witness.as_ref());
        let (witness, coherent) = crate::control::select(view, observation, retained)?;
        crate::control::capability(
            &mut fields,
            witness.is_some(),
            coherent,
            observation.is_some_and(NativeControlObservation::prompt_input_ready),
        );
        observation::apply_interaction(&mut fields, observation, coherent)?;
        let witness_changed = witness.as_ref() != retained.map(|entry| &entry.witness);
        let previous_cursor = previous.map(|entry| entry.snapshot.cursor());
        // Compare before revise moves fields and changes its cursor. Diagnostics
        // retain booleans and public cursors only, never either witness or metadata.
        let public_fields_changed =
            previous.is_some_and(|entry| entry.snapshot.fields() != &fields);
        let snapshot = match previous {
            Some(previous) => crate::control::revise(&previous.snapshot, fields, witness_changed)?,
            None => AgentSnapshot::new(fields)?,
        };
        let process = identity::process(binding).or_else(|| {
            previous
                .filter(|_| same_execution)
                .and_then(|entry| entry.process)
        });
        let entry = Entry {
            subject,
            runtime,
            process,
            detected_at_ms: binding.detected_at_ms,
            updated_at_ms: binding.updated_at_ms,
            source_exited: binding.state == AgentState::Exited
                || previous.is_some_and(|entry| same_execution && entry.source_exited),
            control_witness: witness.map(|witness| crate::control::ControlWitness {
                cursor: if coherent {
                    snapshot.cursor()
                } else {
                    retained.unwrap().cursor
                },
                witness,
            }),
            snapshot: snapshot.clone(),
        };
        let mut next = self.record.clone();
        crate::control_record::retain_previous(&mut next, previous, target)?;
        ensure!(
            selected.is_some()
                || next
                    .entries
                    .len()
                    .saturating_add(next.retired_controls.len())
                    < MAX_AGENT_RECORDS,
            RECORD_LIMIT
        );
        if next.latest.get(&session) != Some(&id) {
            next.invalidate(Some(session))?;
        }
        next.entries.insert(id, entry);
        next.latest.insert(session, id);
        self.commit(next)?;
        if let Some(previous_cursor) = previous_cursor.filter(|cursor| *cursor != snapshot.cursor())
        {
            tracing::debug!(
                message = "Agent projection committed a new observation cursor",
                ?previous_cursor,
                current_cursor = ?snapshot.cursor(),
                coherent,
                witness_changed,
                public_fields_changed,
            );
        }
        Ok(Some(snapshot))
    }
}
