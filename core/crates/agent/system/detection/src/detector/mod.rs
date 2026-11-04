use super::constants::{BINDING_SCHEMA_VERSION, MISSED_DETECTION_THRESHOLD};
use super::{
    AgentBinding, AgentKind, AgentRecoveryAvailability, AgentRegistry, AgentState,
    DetectionContext, DetectionResult, ObservedProcess, SessionGenerationPolicy,
};
use crate::model::now_ms;
use anyhow::Result;

pub struct AgentDetector {
    context: DetectionContext,
    registry: AgentRegistry,
    binding: Option<AgentBinding>,
    consecutive_misses: u8,
}

impl AgentDetector {
    pub fn new(
        context: DetectionContext,
        registry: AgentRegistry,
        binding: Option<AgentBinding>,
    ) -> Result<Self> {
        let mut binding = binding;
        if let Some(binding) = binding.as_mut() {
            let policy = registry
                .adapter(&binding.kind)
                .map_or(SessionGenerationPolicy::AuthoritativeOnly, |adapter| {
                    adapter.session_generation_policy()
                });
            policy.normalize_recovery_availability(binding);
        }
        Ok(Self {
            context,
            registry,
            binding,
            consecutive_misses: 0,
        })
    }

    pub fn session_exited(&mut self) -> DetectionResult {
        self.mark_observed_exit()
    }

    pub fn binding(&self) -> Option<&AgentBinding> {
        self.binding.as_ref()
    }

    pub fn synchronize_binding(&mut self, binding: AgentBinding) -> DetectionResult {
        let mut binding = binding;
        let policy = self
            .registry
            .adapter(&binding.kind)
            .map_or(SessionGenerationPolicy::AuthoritativeOnly, |adapter| {
                adapter.session_generation_policy()
            });
        policy.normalize_recovery_availability(&mut binding);
        let identity_changed = self.binding.as_ref().is_none_or(|current| {
            (
                &current.kind,
                &current.provider_session_id,
                &current.executable,
                &current.cwd,
                &current.evidence_path,
                &current.data_home,
            ) != (
                &binding.kind,
                &binding.provider_session_id,
                &binding.executable,
                &binding.cwd,
                &binding.evidence_path,
                &binding.data_home,
            )
        });
        let status_changed = self.binding.as_ref().is_none_or(|current| {
            (
                current.state,
                current.recovery_availability,
                current.process_pid,
                current.process_start_time,
            ) != (
                binding.state,
                binding.recovery_availability,
                binding.process_pid,
                binding.process_start_time,
            )
        });
        self.binding = Some(binding.clone());
        self.consecutive_misses = 0;
        if identity_changed {
            DetectionResult::BindingChanged(binding)
        } else if status_changed {
            DetectionResult::StatusChanged(binding)
        } else {
            DetectionResult::Unchanged
        }
    }

    fn apply_candidate(
        &mut self,
        kind: AgentKind,
        process: &ObservedProcess,
        evidence: super::AgentEvidence,
    ) -> DetectionResult {
        let now = now_ms();
        let policy = self
            .registry
            .adapter(&kind)
            .map(|adapter| adapter.session_generation_policy());
        let recovery_availability = policy
            .map_or(AgentRecoveryAvailability::Unavailable, |policy| {
                policy.observed_availability()
            });
        let (process_pid, process_start_time) = match evidence.state {
            AgentState::Exited => (None, None),
            _ => (Some(process.pid), Some(process.start_time)),
        };
        let detected_at_ms = self.binding.as_ref().map_or(now, |value| {
            let same =
                value.kind == kind && value.provider_session_id == evidence.provider_session_id;
            if same { value.detected_at_ms } else { now }
        });
        let mut candidate = AgentBinding {
            schema_version: BINDING_SCHEMA_VERSION,
            terminal_id: self.context.terminal_id,
            kind,
            provider_session_id: evidence.provider_session_id,
            executable: process.executable.clone(),
            cwd: evidence.cwd.display().to_string(),
            evidence_path: evidence.path.display().to_string(),
            data_home: evidence.data_home.display().to_string(),
            process_pid,
            process_start_time,
            state: evidence.state,
            recovery_availability,
            detected_at_ms,
            updated_at_ms: now,
        };
        policy
            .unwrap_or(SessionGenerationPolicy::AuthoritativeOnly)
            .normalize_recovery_availability(&mut candidate);
        let identity_changed = self.binding.as_ref().is_none_or(|current| {
            (
                &current.kind,
                &current.provider_session_id,
                &current.executable,
                &current.cwd,
                &current.evidence_path,
                &current.data_home,
            ) != (
                &candidate.kind,
                &candidate.provider_session_id,
                &candidate.executable,
                &candidate.cwd,
                &candidate.evidence_path,
                &candidate.data_home,
            )
        });
        let status_changed = self.binding.as_ref().is_none_or(|current| {
            (
                current.state,
                current.recovery_availability,
                current.process_pid,
                current.process_start_time,
            ) != (
                candidate.state,
                candidate.recovery_availability,
                candidate.process_pid,
                candidate.process_start_time,
            )
        });
        self.binding = Some(candidate.clone());
        if identity_changed {
            DetectionResult::BindingChanged(candidate)
        } else if status_changed {
            DetectionResult::StatusChanged(candidate)
        } else {
            DetectionResult::Unchanged
        }
    }
}

/// Provider display title for an observed binding, independent of detector
/// state so a runtime can resolve it off its async worker thread.
pub fn binding_display_title_with_registry(
    binding: &AgentBinding,
    registry: &AgentRegistry,
) -> Option<String> {
    registry.adapter(&binding.kind)?.display_title(binding)
}

#[path = "candidate.rs"]
mod candidate;

include!("module_wiring.rs");
