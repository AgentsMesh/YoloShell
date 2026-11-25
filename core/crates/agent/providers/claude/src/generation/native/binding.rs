use super::ClaudeNativeGeneration;
use crate::agent::{AgentBinding, AgentRecoveryAvailability, AgentState};

impl ClaudeNativeGeneration {
    pub fn recovery_binding(&self, activity_is_current: bool) -> AgentBinding {
        let mut binding = self.binding.clone();
        binding.state = if activity_is_current {
            self.activity
        } else {
            AgentState::Unknown
        };
        binding.recovery_availability = if self.revoked {
            AgentRecoveryAvailability::Unavailable
        } else {
            AgentRecoveryAvailability::Available
        };
        if binding.state == AgentState::Exited {
            binding.process_pid = None;
            binding.process_start_time = None;
        }
        binding
    }

    pub fn authorizes_resume(&self, binding: &AgentBinding) -> bool {
        !self.revoked
            && binding.terminal_id == self.scope.terminal_id
            && binding.kind == self.binding.kind
            && binding.provider_session_id == self.binding.provider_session_id
            && binding.cwd == self.binding.cwd
            && binding.data_home == self.binding.data_home
            && binding.evidence_path == self.binding.evidence_path
            && self.verify_resume_evidence().unwrap_or(false)
    }
}
