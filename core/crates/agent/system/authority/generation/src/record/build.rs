use anyhow::{Result, ensure};
use uuid::Uuid;
use yolo_agent_authority_contract::AuthorityProcessIdentity;
use yolo_agent_contract::{AgentBinding, AgentRecoveryAvailability, AgentState};
use yolo_hooks_runtime::{HookIdentity, Reducer};

use super::HookGenerationRecord;
use crate::{GenerationEvidence, GenerationRequest};
use super::{INITIAL_REVISION, INVALID, SCHEMA_VERSION};
use crate::apply;

impl HookGenerationRecord {
    pub(crate) fn first(
        request: &GenerationRequest,
        worker: AuthorityProcessIdentity,
        boot_id: &str,
        evidence: &GenerationEvidence,
    ) -> Result<Self> {
        ensure!(
            request.event_sequence == yolo_hooks_runtime::FIRST_SEQUENCE
                && matches!(
                    request.event,
                    yolo_hooks_runtime::EventKind::SessionStart { .. }
                ),
            INVALID
        );
        let now = yolo_terminal_model::now_ms();
        let invocation_id = Uuid::new_v4();
        let hook_token = Uuid::new_v4();
        let identity = HookIdentity {
            terminal_id: request.scope.terminal_id,
            runtime_epoch: request.scope.runtime_epoch,
            invocation_id,
            hook_token,
            process: request.scope.process(),
            generation: yolo_hooks_runtime::Generation::INITIAL,
        };
        let mut reducer = Reducer::new(
            identity.clone(),
            request.provider.clone(),
            request.provider_version,
        )?;
        let mut record = Self {
            schema_version: SCHEMA_VERSION,
            scope: request.scope,
            provider: request.provider.clone(),
            provider_version: request.provider_version,
            worker,
            boot_id: boot_id.to_owned(),
            invocation_id,
            hook_token,
            revision: u64::default(),
            generation: yolo_hooks_runtime::INITIAL_GENERATION,
            last_event_sequence: u64::default(),
            provider_session_id: request.provider_session_id.clone(),
            binding: AgentBinding {
                schema_version: yolo_agent_adapter_contract::constants::BINDING_SCHEMA_VERSION,
                terminal_id: request.scope.terminal_id,
                kind: evidence.kind.clone(),
                provider_session_id: request.provider_session_id.clone(),
                executable: evidence.executable.clone(),
                cwd: evidence.cwd.to_string_lossy().into_owned(),
                evidence_path: evidence.transcript.to_string_lossy().into_owned(),
                data_home: evidence.data_home.to_string_lossy().into_owned(),
                process_pid: Some(request.scope.agent.pid),
                process_start_time: Some(request.scope.agent.start_time),
                state: AgentState::Unknown,
                recovery_availability: AgentRecoveryAvailability::Unavailable,
                detected_at_ms: now,
                updated_at_ms: now,
            },
            state: AgentState::Unknown,
            final_session_end: false,
            revoked: false,
            transcript_identity: evidence.transcript_identity,
            cwd_identity: evidence.cwd_identity,
            data_home_identity: evidence.data_home_identity,
            snapshot: reducer.snapshot()?,
        };
        let disposition = reducer.apply(record.envelope(request))?;
        ensure!(
            disposition == yolo_hooks_runtime::ApplyDisposition::Applied,
            INVALID
        );
        record.capture(&reducer, request, evidence)?;
        record.revision = INITIAL_REVISION;
        Ok(record)
    }

    pub(crate) fn envelope(&self, request: &GenerationRequest) -> yolo_hooks_runtime::HookEnvelope {
        yolo_hooks_runtime::HookEnvelope {
            schema_version: yolo_hooks_runtime::CURRENT_SCHEMA_VERSION,
            event_id: request.event_id,
            sequence: yolo_hooks_runtime::EventSequence(request.event_sequence),
            provider: request.provider.clone(),
            provider_version: request.provider_version,
            identity: self.expected_identity(),
            provider_session_id: request.provider_session_id.clone(),
            turn_id: request.turn_id.clone(),
            agent_id: request.agent_id.clone(),
            tool_name: request.tool_name.clone(),
            tool_use_id: request.tool_use_id.clone(),
            stop_hook_active: request.stop_hook_active,
            metadata: request.metadata.clone(),
            event: request.event.clone(),
        }
    }

    pub(crate) fn capture(
        &mut self,
        reducer: &Reducer,
        request: &GenerationRequest,
        evidence: &GenerationEvidence,
    ) -> Result<()> {
        let state = reducer.state();
        let session_id = state.provider_session_id.as_deref().ok_or_else(|| {
            anyhow::anyhow!("hook generation reducer has no provider session")
        })?;
        ensure!(session_id == request.provider_session_id, INVALID);
        ensure!(
            evidence.provider_session_id == request.provider_session_id
                && state
                    .current_cwd
                    .as_deref()
                    .is_none_or(|cwd| cwd == evidence.cwd)
                && state
                    .current_transcript_path
                    .as_deref()
                    .is_none_or(|path| path == evidence.transcript),
            INVALID
        );
        let now = yolo_terminal_model::now_ms();
        let same_session = self.binding.provider_session_id == session_id;
        self.provider_session_id = session_id.to_owned();
        self.binding.provider_session_id = session_id.to_owned();
        self.binding.cwd = evidence.cwd.to_string_lossy().into_owned();
        self.binding.evidence_path = evidence.transcript.to_string_lossy().into_owned();
        self.binding.data_home = evidence.data_home.to_string_lossy().into_owned();
        self.binding.process_pid = Some(self.scope.agent.pid);
        self.binding.process_start_time = Some(self.scope.agent.start_time);
        self.state = apply::activity(state);
        self.binding.state = self.state;
        self.binding.updated_at_ms = now.max(self.binding.updated_at_ms);
        if !same_session {
            self.binding.detected_at_ms = self.binding.updated_at_ms;
        }
        self.final_session_end = state.final_hook_seen;
        self.binding.recovery_availability = if evidence.transcript_identity.is_some() {
            AgentRecoveryAvailability::Available
        } else {
            AgentRecoveryAvailability::Unavailable
        };
        self.transcript_identity = evidence.transcript_identity;
        self.cwd_identity = evidence.cwd_identity;
        self.data_home_identity = evidence.data_home_identity;
        self.last_event_sequence = request.event_sequence;
        self.snapshot = reducer.snapshot()?;
        Ok(())
    }
}
