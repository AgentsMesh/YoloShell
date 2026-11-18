use crate::{DetectionContext, DetectionEnvironment};
use anyhow::Result;
use std::path::{Path, PathBuf};
use yolo_agent_contract::{
    AgentBinding, AgentEvidence, AgentKind, AgentRecoveryAvailability, AgentState, ObservedProcess,
    ResumePlan,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EvidenceDiscoveryPolicy {
    AuthoritativeOnly,
    ScopedStrongHint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionGenerationPolicy {
    AuthoritativeOnly,
    HookRequired,
}

impl SessionGenerationPolicy {
    pub fn observed_availability(self) -> AgentRecoveryAvailability {
        match self {
            Self::AuthoritativeOnly => AgentRecoveryAvailability::Available,
            Self::HookRequired => AgentRecoveryAvailability::UnmanagedLaunch,
        }
    }

    pub fn normalize_recovery_availability(self, binding: &mut AgentBinding) -> bool {
        let changed = Self::requires_unavailable(binding.state)
            && binding.recovery_availability == AgentRecoveryAvailability::Available;
        if changed {
            binding.recovery_availability = AgentRecoveryAvailability::Unavailable;
        }
        changed
    }

    pub fn recovery_availability_is_valid(self, binding: &AgentBinding) -> bool {
        match self {
            Self::HookRequired => matches!(
                (binding.state, binding.recovery_availability),
                (
                    AgentState::Unknown,
                    AgentRecoveryAvailability::UnmanagedLaunch
                        | AgentRecoveryAvailability::Unavailable
                        | AgentRecoveryAvailability::Available
                ) | (
                    AgentState::Active
                        | AgentState::Running
                        | AgentState::WaitingForInput
                        | AgentState::WaitingForPermission,
                    AgentRecoveryAvailability::Available
                ) | (
                    AgentState::Exited,
                    AgentRecoveryAvailability::Available | AgentRecoveryAvailability::Unavailable
                )
            ),
            Self::AuthoritativeOnly if Self::requires_unavailable(binding.state) => {
                binding.recovery_availability == AgentRecoveryAvailability::Unavailable
            }
            Self::AuthoritativeOnly if binding.state == AgentState::Exited => {
                binding.recovery_availability != AgentRecoveryAvailability::UnmanagedLaunch
            }
            Self::AuthoritativeOnly => {
                binding.recovery_availability == AgentRecoveryAvailability::Available
            }
        }
    }

    fn requires_unavailable(state: AgentState) -> bool {
        state == AgentState::Unknown
    }
}

pub trait AgentAdapter: Send + Sync {
    fn kind(&self) -> AgentKind;
    fn data_home_environment_variable(&self) -> Option<&'static str> {
        None
    }
    fn resolve_data_home(&self, _: &DetectionEnvironment) -> Option<PathBuf> {
        None
    }
    fn matches_process(&self, process: &ObservedProcess) -> bool;
    fn owns_evidence_path(&self, _: &Path, _: &DetectionContext) -> bool {
        false
    }
    fn is_evidence_candidate_path(&self, _: &Path) -> bool {
        true
    }
    fn evidence_matches_session_hint(&self, _: &Path, _: &str) -> bool {
        true
    }
    fn evidence_discovery_policy(&self) -> EvidenceDiscoveryPolicy {
        EvidenceDiscoveryPolicy::AuthoritativeOnly
    }
    fn session_generation_policy(&self) -> SessionGenerationPolicy {
        SessionGenerationPolicy::AuthoritativeOnly
    }
    fn process_session_arguments_valid(&self, _: &ObservedProcess) -> bool {
        true
    }
    fn process_session_id(&self, _: &ObservedProcess) -> Option<String> {
        None
    }
    fn process_resume_session_id(&self, _: &ObservedProcess) -> Option<String> {
        None
    }
    fn evidence_files(&self, context: &DetectionContext) -> Result<Vec<PathBuf>>;
    fn discover_evidence(&self, context: &DetectionContext, _: &Path) -> Result<Vec<PathBuf>> {
        self.evidence_files(context)
    }
    fn inspect_evidence(&self, path: &Path, agent_cwd: &Path) -> Result<Option<AgentEvidence>>;
    fn inspect_evidence_in_context(
        &self,
        path: &Path,
        agent_cwd: &Path,
        _: &DetectionContext,
    ) -> Result<Option<AgentEvidence>> {
        self.inspect_evidence(path, agent_cwd)
    }
    fn contextualize_evidence(
        &self,
        _: &ObservedProcess,
        evidence: AgentEvidence,
    ) -> Result<Option<AgentEvidence>> {
        Ok(Some(evidence))
    }
    fn requires_evidence_cwd_match(&self) -> bool {
        true
    }
    /// Best-effort provider display title for an agent binding. This is
    /// presentation metadata only; it never participates in binding identity,
    /// recovery authority or control admission.
    fn display_title(&self, _: &AgentBinding) -> Option<String> {
        None
    }
    fn resume_plan(&self, binding: &AgentBinding) -> ResumePlan;
    fn validate_binding(&self, binding: &AgentBinding, context: &DetectionContext) -> Result<()>;
}
