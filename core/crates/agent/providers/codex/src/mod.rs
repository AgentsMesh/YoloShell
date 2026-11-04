mod argv;
mod authoritative_evidence;
pub use authoritative_evidence::AuthoritativeCodexEvidence;
#[doc(hidden)]
pub mod authority;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[doc(hidden)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/authority_test_hook.rs"]
pub mod authority_test_hook;
pub mod constants;
mod cwd;
mod evidence_identity;
mod evidence_open;
mod evidence_path;
mod evidence_reader;
mod exact_resume;
mod inspection;
mod turn_end;
pub use turn_end::{TurnEnd, TurnEndKind};
mod metadata;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
include!("../unit_tests/module_wiring.rs");

#[cfg(any(test, feature = "yolo-unit-tests", feature = "yolo-integration-tests"))]
use super::AgentState;
use super::adapter::{AgentAdapter, EvidenceDiscoveryPolicy, SessionGenerationPolicy};
use super::files::collect_jsonl;
use super::{
    AgentBinding, AgentEvidence, AgentKind, DetectionContext, DetectionEnvironment,
    ObservedProcess, ResumePlan,
};
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct CodexAdapter;

impl AgentAdapter for CodexAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }

    fn data_home_environment_variable(&self) -> Option<&'static str> {
        Some(constants::DATA_HOME_ENV)
    }

    fn resolve_data_home(&self, environment: &DetectionEnvironment) -> Option<PathBuf> {
        environment.data_home(constants::DATA_HOME_ENV, constants::DEFAULT_DATA_HOME_DIR)
    }

    fn matches_process(&self, process: &ObservedProcess) -> bool {
        super::matching::matches_executable_or_node_entrypoint(
            process,
            constants::EXECUTABLE_NAMES,
            constants::NODE_PACKAGE_COMPONENTS,
            constants::NODE_ENTRYPOINTS,
        )
    }

    fn owns_evidence_path(&self, path: &Path, context: &DetectionContext) -> bool {
        context.data_home(&self.kind()).is_some_and(|home| {
            super::validation::evidence_path_is_owned(path, &home.join(constants::EVIDENCE_DIR))
        })
    }

    fn is_evidence_candidate_path(&self, path: &Path) -> bool {
        metadata::main_rollout_path(path)
    }

    fn evidence_matches_session_hint(&self, path: &Path, hint: &str) -> bool {
        metadata::rollout_session_id(path) == Some(hint)
    }

    fn evidence_discovery_policy(&self) -> EvidenceDiscoveryPolicy {
        EvidenceDiscoveryPolicy::ScopedStrongHint
    }

    fn session_generation_policy(&self) -> SessionGenerationPolicy {
        SessionGenerationPolicy::HookRequired
    }

    fn process_session_arguments_valid(&self, process: &ObservedProcess) -> bool {
        argv::session_identity(process).is_valid()
    }

    fn process_session_id(&self, process: &ObservedProcess) -> Option<String> {
        argv::session_identity(process).into_session_id()
    }

    fn process_resume_session_id(&self, process: &ObservedProcess) -> Option<String> {
        exact_resume::session_id(process)
    }

    fn evidence_files(&self, context: &DetectionContext) -> Result<Vec<PathBuf>> {
        let Some(home) = context.data_home(&self.kind()) else {
            return Ok(Vec::new());
        };
        collect_jsonl(&home.join(constants::EVIDENCE_DIR))
    }

    fn inspect_evidence(&self, path: &Path, agent_cwd: &Path) -> Result<Option<AgentEvidence>> {
        let _ = (path, agent_cwd);
        Ok(None)
    }

    fn inspect_evidence_in_context(
        &self,
        path: &Path,
        agent_cwd: &Path,
        context: &DetectionContext,
    ) -> Result<Option<AgentEvidence>> {
        let Some(data_home) = context.data_home(&self.kind()) else {
            return Ok(None);
        };
        self.inspect_evidence_with_hook(path, agent_cwd, data_home, || {})
    }

    fn contextualize_evidence(
        &self,
        process: &ObservedProcess,
        mut evidence: AgentEvidence,
    ) -> Result<Option<AgentEvidence>> {
        match cwd::process_override(process) {
            cwd::CwdOverride::Absent => {
                let Some(cwd) = metadata::canonical_existing_cwd(&evidence.cwd) else {
                    return Ok(None);
                };
                evidence.cwd = cwd;
            }
            cwd::CwdOverride::Invalid => return Ok(None),
            cwd::CwdOverride::Resolved(cwd) => evidence.cwd = cwd,
        }
        Ok(Some(evidence))
    }

    fn requires_evidence_cwd_match(&self) -> bool {
        false
    }

    fn display_title(&self, binding: &AgentBinding) -> Option<String> {
        metadata::display_title(Path::new(&binding.data_home), &binding.provider_session_id)
    }

    fn resume_plan(&self, binding: &AgentBinding) -> ResumePlan {
        let environment = (!binding.data_home.is_empty())
            .then(|| (constants::DATA_HOME_ENV.into(), binding.data_home.clone()))
            .into_iter()
            .collect();
        ResumePlan {
            executable: constants::EXECUTABLE.to_owned(),
            args: vec![
                constants::RESUME_ARGUMENT.to_owned(),
                constants::CWD_ARGUMENT.to_owned(),
                binding.cwd.clone(),
                binding.provider_session_id.clone(),
            ],
            environment,
        }
    }

    fn validate_binding(&self, binding: &AgentBinding, context: &DetectionContext) -> Result<()> {
        anyhow::ensure!(
            metadata::canonical_session_id(&binding.provider_session_id).is_some(),
            constants::SESSION_ID_INVALID_ERROR
        );
        let data_home = Path::new(&binding.data_home);
        let cwd = Path::new(&binding.cwd);
        anyhow::ensure!(
            metadata::canonical_existing_cwd(cwd).as_deref() == Some(cwd),
            super::constants::validation::CWD
        );
        anyhow::ensure!(
            metadata::canonical_existing_cwd(data_home).as_deref() == Some(data_home),
            super::constants::validation::DATA_HOME
        );
        super::validation::validate_adapter_binding(
            binding,
            context,
            self.kind(),
            constants::EXECUTABLE_NAMES,
            data_home,
            &data_home.join(constants::EVIDENCE_DIR),
        )
    }
}
