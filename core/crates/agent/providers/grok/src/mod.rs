mod argv;
pub mod constants;
mod evidence;
pub use evidence::AuthoritativeGrokEvidence;
mod metadata;

use super::adapter::{
    AgentAdapter, DetectionContext, DetectionEnvironment, EvidenceDiscoveryPolicy,
    SessionGenerationPolicy,
};
use super::{AgentBinding, AgentEvidence, AgentKind, ObservedProcess, ResumePlan};
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct GrokAdapter;

/// Canonical data home for a live agent environment: `$GROK_HOME` or
/// `$HOME/.grok`, canonicalized so it matches transcript paths.
pub fn data_home(environment: &BTreeMap<String, String>) -> Option<PathBuf> {
    metadata::data_home_from_environment(environment)
        .and_then(|home| home.canonicalize().ok())
}

impl AgentAdapter for GrokAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Other(constants::KIND.into())
    }

    fn data_home_environment_variable(&self) -> Option<&'static str> {
        Some(constants::DATA_HOME_ENV)
    }

    fn resolve_data_home(&self, environment: &DetectionEnvironment) -> Option<PathBuf> {
        // GROK_HOME names the base directory itself, unlike the home wrappers
        // whose variable names the parent of a fixed child directory.
        environment
            .variable(constants::DATA_HOME_ENV)
            .or_else(|| {
                environment
                    .variable(constants::HOME_ENV)
                    .map(|home| home.join(constants::DATA_HOME_DIR))
            })
    }

    fn matches_process(&self, process: &ObservedProcess) -> bool {
        argv::matches_executable(process)
    }

    fn owns_evidence_path(&self, path: &Path, context: &DetectionContext) -> bool {
        context.data_home(&self.kind()).is_some_and(|home| {
            crate::agent::adapter::validation::evidence_path_is_owned(
                path,
                &home.join(constants::EVIDENCE_DIR),
            )
        })
    }

    fn is_evidence_candidate_path(&self, path: &Path) -> bool {
        metadata::main_transcript_path(path)
    }

    fn evidence_matches_session_hint(&self, path: &Path, hint: &str) -> bool {
        metadata::transcript_session_id(path).as_deref() == Some(hint)
    }

    fn evidence_discovery_policy(&self) -> EvidenceDiscoveryPolicy {
        EvidenceDiscoveryPolicy::ScopedStrongHint
    }

    fn session_generation_policy(&self) -> SessionGenerationPolicy {
        SessionGenerationPolicy::HookRequired
    }

    fn process_session_arguments_valid(&self, process: &ObservedProcess) -> bool {
        argv::command_arguments(process)
            .is_some_and(|arguments| argv::session_identity(arguments).is_valid())
    }

    fn process_session_id(&self, process: &ObservedProcess) -> Option<String> {
        argv::command_arguments(process)
            .and_then(|arguments| argv::session_identity(arguments).into_session_id())
    }

    fn process_resume_session_id(&self, process: &ObservedProcess) -> Option<String> {
        argv::command_arguments(process).and_then(argv::resume_session_id)
    }

    fn evidence_files(&self, context: &DetectionContext) -> Result<Vec<PathBuf>> {
        let Some(home) = context.data_home(&self.kind()) else {
            return Ok(Vec::new());
        };
        let root = home.join(constants::EVIDENCE_DIR);
        if !root.is_dir() {
            return Ok(Vec::new());
        }
        let mut files = Vec::new();
        collect_transcripts(&root, &mut files);
        files.sort();
        Ok(files)
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
        Ok(self
            .inspect_authoritative_evidence_with_hook(path, agent_cwd, &data_home, || {})?
            .map(|evidence| evidence.evidence().clone()))
    }

    fn requires_evidence_cwd_match(&self) -> bool {
        false
    }

    fn resume_plan(&self, binding: &AgentBinding) -> ResumePlan {
        let environment = metadata::resume_environment(&binding.data_home)
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        ResumePlan {
            executable: constants::EXECUTABLE.to_owned(),
            args: vec![
                constants::SHORT_RESUME_ARGUMENT.to_owned(),
                binding.provider_session_id.clone(),
            ],
            environment,
        }
    }

    fn validate_binding(&self, binding: &AgentBinding, context: &DetectionContext) -> Result<()> {
        anyhow::ensure!(
            metadata::normalized_session_id(&binding.provider_session_id).is_some(),
            constants::INVALID_SESSION_ID
        );
        let data_home = Path::new(&binding.data_home);
        let cwd = Path::new(&binding.cwd);
        anyhow::ensure!(
            metadata::canonical_existing_cwd(cwd).as_deref() == Some(cwd),
            constants::INVALID_CWD
        );
        anyhow::ensure!(
            metadata::canonical_existing_cwd(data_home).as_deref() == Some(data_home),
            constants::INVALID_DATA_HOME
        );
        crate::agent::adapter::validation::validate_adapter_binding(
            binding,
            context,
            self.kind(),
            constants::EXECUTABLE_NAMES,
            data_home,
            &data_home.join(constants::EVIDENCE_DIR),
        )
    }
}

fn collect_transcripts(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_transcripts(&path, files);
        } else if metadata::main_transcript_path(&path) {
            files.push(path);
        }
    }
}
