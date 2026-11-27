use super::super::*;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::tempdir;
use uuid::Uuid;
use yolo_codex::codex;

pub(super) const ORIGINAL_START: u64 = 101;
pub(super) const REUSED_START: u64 = 202;
pub(super) const LATEST_START: u64 = 303;
pub(super) const ORIGINAL_SESSION: &str = "original-session";
pub(super) const REUSED_SESSION: &str = "reused-session";
const PROCESS_PID: u32 = 731;
const PROCESS_GROUP: i32 = 97;
const FILESYSTEM_TIMESTAMP_TICK: Duration = Duration::from_millis(2);

struct IdentityAdapter {
    root: PathBuf,
    paths: Vec<PathBuf>,
    inspected: Arc<Mutex<Vec<PathBuf>>>,
}

impl AgentAdapter for IdentityAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }

    fn matches_process(&self, _: &ObservedProcess) -> bool {
        true
    }

    fn owns_evidence_path(&self, path: &Path, _: &DetectionContext) -> bool {
        path.starts_with(&self.root)
    }

    fn evidence_files(&self, _: &DetectionContext) -> Result<Vec<PathBuf>> {
        Ok(self.paths.clone())
    }

    fn inspect_evidence(&self, path: &Path, cwd: &Path) -> Result<Option<AgentEvidence>> {
        self.inspected.lock().unwrap().push(path.into());
        Ok(Some(AgentEvidence {
            provider_session_id: std::fs::read_to_string(path)?,
            cwd: cwd.into(),
            path: path.into(),
            data_home: self.root.clone(),
            state: AgentState::WaitingForInput,
        }))
    }

    fn resume_plan(&self, _: &AgentBinding) -> ResumePlan {
        ResumePlan {
            executable: codex::constants::EXECUTABLE.into(),
            args: Vec::new(),
            environment: Default::default(),
        }
    }

    fn validate_binding(&self, _: &AgentBinding, _: &DetectionContext) -> Result<()> {
        Ok(())
    }
}

pub(super) struct Fixture {
    _directory: tempfile::TempDir,
    cwd: PathBuf,
    pub(super) original: PathBuf,
    pub(super) reused: PathBuf,
    pub(super) inspected: Arc<Mutex<Vec<PathBuf>>>,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let directory = tempdir().unwrap();
        let cwd = directory.path().join("project");
        let root = directory.path().join("evidence");
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::create_dir_all(&root).unwrap();
        let original = root.join("original.jsonl");
        let reused = root.join("reused.jsonl");
        std::fs::write(&original, ORIGINAL_SESSION).unwrap();
        std::fs::write(&reused, REUSED_SESSION).unwrap();
        Self {
            _directory: directory,
            cwd,
            original,
            reused,
            inspected: Default::default(),
        }
    }

    pub(super) fn detector(&self) -> AgentDetector {
        std::thread::sleep(FILESYSTEM_TIMESTAMP_TICK);
        let root = self.original.parent().unwrap().to_path_buf();
        let adapter = IdentityAdapter {
            root: root.clone(),
            paths: vec![self.original.clone(), self.reused.clone()],
            inspected: Arc::clone(&self.inspected),
        };
        AgentDetector::new(
            DetectionContext::new(
                Uuid::new_v4(),
                self.cwd.clone(),
                [(AgentKind::Codex, root.clone())],
            ),
            AgentRegistry::from_adapters(vec![Box::new(adapter)]).unwrap(),
            Some(self.binding(root)),
        )
        .unwrap()
    }

    pub(super) fn refresh_reused_evidence(&self) {
        std::thread::sleep(FILESYSTEM_TIMESTAMP_TICK);
        std::fs::write(&self.reused, REUSED_SESSION).unwrap();
    }

    fn binding(&self, root: PathBuf) -> AgentBinding {
        AgentBinding {
            schema_version: constants::BINDING_SCHEMA_VERSION,
            terminal_id: Uuid::new_v4(),
            kind: AgentKind::Codex,
            provider_session_id: ORIGINAL_SESSION.into(),
            executable: codex::constants::EXECUTABLE.into(),
            cwd: self.cwd.display().to_string(),
            evidence_path: self.original.display().to_string(),
            data_home: root.display().to_string(),
            process_pid: Some(PROCESS_PID),
            process_start_time: Some(ORIGINAL_START),
            state: AgentState::Running,
            recovery_availability: crate::agent::AgentRecoveryAvailability::Available,
            detected_at_ms: 1,
            updated_at_ms: 2,
        }
    }

    pub(super) fn process(&self, start_time: u64, paths: Option<Vec<PathBuf>>) -> ObservedProcess {
        ObservedProcess {
            pid: PROCESS_PID,
            start_time,
            parent_pid: 1,
            process_group: PROCESS_GROUP,
            foreground_group: PROCESS_GROUP,
            executable: codex::constants::EXECUTABLE.into(),
            argv: vec![codex::constants::EXECUTABLE.into()],
            environment: None,
            cwd: Some(self.cwd.clone()),
            evidence_paths: paths,
        }
    }
}
