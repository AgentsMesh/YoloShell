use super::*;
use anyhow::Result;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(super) const BOUND_SESSION_ID: &str = "session";
pub(super) const DIFFERENT_SESSION_ID: &str = "different-session";
pub(super) const PROCESS_PID: u32 = 21;
pub(super) const OTHER_PROCESS_PID: u32 = 22;
pub(super) const CHANGED_PROCESS_PID: u32 = 99;
const DISCOVERY_SESSION_ID: &str = "discovery-session";
const PARENT_PID: u32 = 1;
const PROCESS_GROUP: i32 = 21;

struct StaticAdapter {
    path: PathBuf,
}

impl super::super::AgentAdapter for StaticAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }

    fn matches_process(&self, _: &ObservedProcess) -> bool {
        true
    }

    fn evidence_discovery_policy(&self) -> super::super::EvidenceDiscoveryPolicy {
        super::super::EvidenceDiscoveryPolicy::ScopedStrongHint
    }

    fn process_session_id(&self, process: &ObservedProcess) -> Option<String> {
        process
            .argv
            .last()
            .filter(|_| process.argv.len() > 1)
            .cloned()
    }

    fn evidence_files(&self, _: &DetectionContext) -> Result<Vec<PathBuf>> {
        Ok(vec![self.path.clone()])
    }

    fn inspect_evidence(&self, _: &Path, _: &Path) -> Result<Option<super::super::AgentEvidence>> {
        Ok(None)
    }

    fn resume_plan(&self, _: &AgentBinding) -> super::super::ResumePlan {
        super::super::ResumePlan {
            executable: crate::agent::codex::constants::EXECUTABLE.into(),
            args: vec![
                crate::agent::codex::constants::RESUME_ARGUMENT.into(),
                BOUND_SESSION_ID.into(),
            ],
            environment: Default::default(),
        }
    }

    fn validate_binding(&self, _: &AgentBinding, _: &DetectionContext) -> Result<()> {
        Ok(())
    }
}

pub(super) fn registry(path: PathBuf) -> AgentRegistry {
    AgentRegistry::from_adapters(vec![Box::new(StaticAdapter { path })]).unwrap()
}

pub(super) fn context(cwd: &Path) -> DetectionContext {
    DetectionContext::new(Uuid::new_v4(), cwd.into(), [(AgentKind::Codex, cwd.into())])
}

pub(super) fn process(cwd: Option<&Path>, pid: u32) -> ObservedProcess {
    ObservedProcess {
        pid,
        start_time: u64::from(pid),
        parent_pid: PARENT_PID,
        process_group: PROCESS_GROUP,
        foreground_group: PROCESS_GROUP,
        executable: crate::agent::codex::constants::EXECUTABLE.into(),
        argv: vec![
            crate::agent::codex::constants::EXECUTABLE.into(),
            DISCOVERY_SESSION_ID.into(),
        ],
        environment: None,
        cwd: cwd.map(Path::to_path_buf),
        evidence_paths: None,
    }
}

pub(super) fn binding(kind: AgentKind, path: &Path) -> AgentBinding {
    AgentBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        kind,
        provider_session_id: BOUND_SESSION_ID.into(),
        executable: crate::agent::codex::constants::EXECUTABLE.into(),
        cwd: "/workspace".into(),
        evidence_path: path.display().to_string(),
        data_home: "/home".into(),
        process_pid: Some(PROCESS_PID),
        process_start_time: Some(u64::from(PROCESS_PID)),
        state: AgentState::Running,
        recovery_availability: crate::agent::AgentRecoveryAvailability::Available,
        detected_at_ms: 1,
        updated_at_ms: 2,
    }
}

pub(super) fn evidence(path: &Path, cwd: &Path, id: &str) -> super::super::AgentEvidence {
    super::super::AgentEvidence {
        provider_session_id: id.into(),
        cwd: cwd.into(),
        path: path.into(),
        data_home: path.parent().unwrap().into(),
        state: AgentState::Running,
    }
}
