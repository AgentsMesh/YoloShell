mod argv;
mod authoritative_evidence;
#[doc(hidden)]
pub use authoritative_evidence::named_transcript_identity as durable_transcript_identity;
pub mod constants;
pub use authoritative_evidence::AuthoritativeClaudeEvidence;
mod discovery;
mod evidence_identity;
mod evidence_open;
mod evidence_path;
mod evidence_reader;
pub mod generation;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[doc(hidden)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/generation_test_hook.rs"]
pub mod generation_test_hook;
mod inspection;
mod metadata;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
include!("../unit_tests/module_wiring.rs");
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../test_support/real_schema_integration.rs"]
mod real_schema_integration;

#[cfg(any(test, feature = "yolo-unit-tests", feature = "yolo-integration-tests"))]
use super::AgentState;
use super::adapter::{AgentAdapter, EvidenceDiscoveryPolicy, SessionGenerationPolicy};
use super::{
    AgentBinding, AgentEvidence, AgentKind, DetectionContext, DetectionEnvironment,
    ObservedProcess, ResumePlan,
};
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct ClaudeAdapter;

impl AgentAdapter for ClaudeAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::ClaudeCode
    }

    fn data_home_environment_variable(&self) -> Option<&'static str> {
        Some(constants::DATA_HOME_ENV)
    }

    fn resolve_data_home(&self, environment: &DetectionEnvironment) -> Option<PathBuf> {
        environment.data_home(constants::DATA_HOME_ENV, constants::DEFAULT_DATA_HOME_DIR)
    }

    fn matches_process(&self, process: &ObservedProcess) -> bool {
        if internal_helper(process) {
            return false;
        }
        super::matching::matches_executable_or_node_entrypoint(
            process,
            constants::EXECUTABLE_NAMES,
            constants::NODE_PACKAGE_COMPONENTS,
            constants::NODE_ENTRYPOINTS,
        )
    }

    fn owns_evidence_path(&self, path: &Path, context: &DetectionContext) -> bool {
        self.data_home(context).is_some_and(|home| {
            super::validation::evidence_path_is_owned(path, &home.join(constants::EVIDENCE_DIR))
        })
    }

    fn is_evidence_candidate_path(&self, path: &Path) -> bool {
        metadata::main_transcript_path(path)
    }

    fn evidence_discovery_policy(&self) -> EvidenceDiscoveryPolicy {
        EvidenceDiscoveryPolicy::ScopedStrongHint
    }

    fn session_generation_policy(&self) -> SessionGenerationPolicy {
        SessionGenerationPolicy::HookRequired
    }

    fn process_session_arguments_valid(&self, process: &ObservedProcess) -> bool {
        argv::session_identity(&process.argv).is_valid()
    }

    fn process_session_id(&self, process: &ObservedProcess) -> Option<String> {
        argv::session_identity(&process.argv).into_session_id()
    }

    fn process_resume_session_id(&self, process: &ObservedProcess) -> Option<String> {
        argv::resume_session_id(&process.argv)
    }

    fn evidence_files(&self, context: &DetectionContext) -> Result<Vec<PathBuf>> {
        let Some(home) = self.data_home(context) else {
            return Ok(Vec::new());
        };
        discovery::evidence_files(&home.join(constants::EVIDENCE_DIR), &context.expected_cwd)
    }

    fn discover_evidence(
        &self,
        context: &DetectionContext,
        agent_cwd: &Path,
    ) -> Result<Vec<PathBuf>> {
        let Some(home) = self.data_home(context) else {
            return Ok(Vec::new());
        };
        discovery::evidence_files(&home.join(constants::EVIDENCE_DIR), agent_cwd)
    }

    fn inspect_evidence(&self, path: &Path, agent_cwd: &Path) -> Result<Option<AgentEvidence>> {
        self.inspect_evidence_with_hook(path, agent_cwd, || {})
    }

    fn inspect_evidence_in_context(
        &self,
        path: &Path,
        agent_cwd: &Path,
        context: &DetectionContext,
    ) -> Result<Option<AgentEvidence>> {
        let Some(data_home) = self.data_home(context) else {
            return Ok(None);
        };
        self.inspect_evidence_at(path, agent_cwd, data_home, || {})
    }

    fn display_title(&self, binding: &AgentBinding) -> Option<String> {
        let path = Path::new(&binding.evidence_path);
        let data_home = Path::new(&binding.data_home);
        let window = evidence_reader::read_with_hook(data_home, path, || {})
            .ok()
            .flatten()?;
        metadata::display_title(&window.values, &binding.provider_session_id)
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
                binding.provider_session_id.clone(),
            ],
            environment,
        }
    }

    fn validate_binding(&self, binding: &AgentBinding, context: &DetectionContext) -> Result<()> {
        let data_home = Path::new(&binding.data_home);
        super::validation::validate_adapter_binding(
            binding,
            context,
            self.kind(),
            constants::EXECUTABLE_NAMES,
            data_home,
            &data_home.join(constants::EVIDENCE_DIR),
        )?;
        anyhow::ensure!(
            metadata::main_transcript_path(Path::new(&binding.evidence_path)),
            super::constants::validation::EVIDENCE_PATH
        );
        Ok(())
    }
}

fn internal_helper(process: &ObservedProcess) -> bool {
    matches!(
        process.argv.as_slice(),
        [_, argument] if crate::claude_hooks::is_internal_argument(argument)
    ) || matches!(
        process.argv.as_slice(),
        [_, argument, _] if argument == crate::claude_hooks::CLAUDE_HOOK_HELPER_ARGUMENT
    )
}

impl ClaudeAdapter {
    fn data_home<'a>(&self, context: &'a DetectionContext) -> Option<&'a Path> {
        context.data_home(&self.kind())
    }
}
