use super::AgentDetector;
use crate::agent::{
    AgentRecoveryAvailability, AgentState, DetectionResult, SessionGenerationPolicy,
};
use crate::model::now_ms;

impl AgentDetector {
    pub(super) fn mark_observed_exit(&mut self) -> DetectionResult {
        if binding_policy(self) == SessionGenerationPolicy::AuthoritativeOnly {
            return self.mark_exited();
        }
        let Some(binding) = self.binding.as_mut() else {
            return DetectionResult::Unchanged;
        };
        if binding.state == AgentState::Exited
            && binding.process_pid.is_none()
            && binding.process_start_time.is_none()
        {
            return DetectionResult::Unchanged;
        }
        let changed = binding.state != AgentState::Unknown
            || binding.recovery_availability != AgentRecoveryAvailability::Unavailable;
        binding.state = AgentState::Unknown;
        binding.recovery_availability = AgentRecoveryAvailability::Unavailable;
        if !changed {
            return DetectionResult::Unchanged;
        }
        binding.updated_at_ms = now_ms();
        DetectionResult::StatusChanged(binding.clone())
    }

    pub(super) fn mark_exited(&mut self) -> DetectionResult {
        let policy = binding_policy(self);
        let Some(binding) = self.binding.as_mut() else {
            return DetectionResult::Unchanged;
        };
        let lifecycle_changed = binding.state != AgentState::Exited
            || binding.process_pid.is_some()
            || binding.process_start_time.is_some();
        binding.state = AgentState::Exited;
        binding.process_pid = None;
        binding.process_start_time = None;
        let availability_changed = policy.normalize_recovery_availability(binding);
        if !lifecycle_changed && !availability_changed {
            return DetectionResult::Unchanged;
        }
        binding.updated_at_ms = now_ms();
        DetectionResult::StatusChanged(binding.clone())
    }

    pub(super) fn mark_unknown(&mut self) -> DetectionResult {
        let policy = binding_policy(self);
        let Some(binding) = self.binding.as_mut() else {
            return DetectionResult::Unchanged;
        };
        let state_changed = binding.state != AgentState::Unknown;
        binding.state = AgentState::Unknown;
        let availability_changed = policy.normalize_recovery_availability(binding);
        if !state_changed && !availability_changed {
            return DetectionResult::Unchanged;
        }
        binding.updated_at_ms = now_ms();
        DetectionResult::StatusChanged(binding.clone())
    }

    pub(super) fn note_missing(&mut self) -> DetectionResult {
        if self.binding.is_none() {
            return DetectionResult::Unchanged;
        }
        self.consecutive_misses = self.consecutive_misses.saturating_add(1);
        if self.consecutive_misses < super::MISSED_DETECTION_THRESHOLD {
            DetectionResult::Unchanged
        } else {
            self.mark_observed_exit()
        }
    }
}

pub(super) fn binding_policy(detector: &AgentDetector) -> SessionGenerationPolicy {
    detector
        .binding
        .as_ref()
        .and_then(|binding| detector.registry.adapter(&binding.kind))
        .map_or(SessionGenerationPolicy::AuthoritativeOnly, |adapter| {
            adapter.session_generation_policy()
        })
}
