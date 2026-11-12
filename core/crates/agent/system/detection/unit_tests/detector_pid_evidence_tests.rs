use super::*;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::tempdir;
use uuid::Uuid;

const FIRST_SESSION: &str = "first-session";
const SECOND_SESSION: &str = "second-session";
const PROCESS_PID: u32 = 731;
const PROCESS_GROUP: i32 = 97;
const UNAVAILABLE_EVIDENCE_FILE: &str = "unavailable.jsonl";

struct AssociatedAdapter {
    root: PathBuf,
    fallback: Vec<PathBuf>,
    scans: Arc<AtomicUsize>,
    inspections: Arc<AtomicUsize>,
}

impl AgentAdapter for AssociatedAdapter {
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
        self.scans.fetch_add(1, Ordering::SeqCst);
        Ok(self.fallback.clone())
    }

    fn inspect_evidence(&self, path: &Path, cwd: &Path) -> Result<Option<AgentEvidence>> {
        self.inspections.fetch_add(1, Ordering::SeqCst);
        Ok(Some(AgentEvidence {
            provider_session_id: fs::read_to_string(path)?,
            cwd: cwd.into(),
            path: path.into(),
            data_home: self.root.clone(),
            state: AgentState::Running,
        }))
    }

    fn resume_plan(&self, _: &AgentBinding) -> ResumePlan {
        ResumePlan {
            executable: crate::agent::codex::constants::EXECUTABLE.into(),
            args: Vec::new(),
            environment: Default::default(),
        }
    }

    fn validate_binding(&self, _: &AgentBinding, _: &DetectionContext) -> Result<()> {
        Ok(())
    }
}

struct Fixture {
    _directory: tempfile::TempDir,
    shell_cwd: PathBuf,
    agent_cwd: PathBuf,
    root: PathBuf,
    first: PathBuf,
    second: PathBuf,
    scans: Arc<AtomicUsize>,
    inspections: Arc<AtomicUsize>,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().unwrap();
        let shell_cwd = directory.path().join("shell");
        let agent_cwd = directory.path().join("agent");
        let root = directory.path().join("evidence");
        fs::create_dir_all(&shell_cwd).unwrap();
        fs::create_dir_all(&agent_cwd).unwrap();
        fs::create_dir_all(&root).unwrap();
        let first = root.join("first.jsonl");
        let second = root.join("second.jsonl");
        fs::write(&first, FIRST_SESSION).unwrap();
        fs::write(&second, SECOND_SESSION).unwrap();
        Self {
            _directory: directory,
            shell_cwd,
            agent_cwd,
            root,
            first,
            second,
            scans: Arc::new(AtomicUsize::new(0)),
            inspections: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn detector(&self) -> AgentDetector {
        let adapter = AssociatedAdapter {
            root: self.root.clone(),
            fallback: vec![self.first.clone(), self.second.clone()],
            scans: Arc::clone(&self.scans),
            inspections: Arc::clone(&self.inspections),
        };
        AgentDetector::new(
            DetectionContext::new(
                Uuid::new_v4(),
                self.shell_cwd.clone(),
                [(AgentKind::Codex, self.root.clone())],
            ),
            AgentRegistry::from_adapters(vec![Box::new(adapter)]).unwrap(),
            None,
        )
        .unwrap()
    }

    fn process(&self, paths: Option<Vec<PathBuf>>) -> ObservedProcess {
        ObservedProcess {
            pid: PROCESS_PID,
            start_time: u64::from(PROCESS_PID),
            parent_pid: 1,
            process_group: PROCESS_GROUP,
            foreground_group: PROCESS_GROUP,
            executable: crate::agent::codex::constants::EXECUTABLE.into(),
            argv: vec![crate::agent::codex::constants::EXECUTABLE.into()],
            environment: None,
            cwd: Some(self.agent_cwd.clone()),
            evidence_paths: paths,
        }
    }
}

#[test]
fn pid_association_resolves_concurrent_candidates_after_shell_cd() {
    let fixture = Fixture::new();
    let mut detector = fixture.detector();
    let result = detector
        .detect(&[fixture.process(Some(vec![fixture.second.clone()]))])
        .unwrap();
    assert!(matches!(
        result,
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == SECOND_SESSION
                && binding.cwd == fixture.agent_cwd.display().to_string()
    ));
    assert_eq!(fixture.scans.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.inspections.load(Ordering::SeqCst), 1);
}

#[test]
fn available_empty_inventory_does_not_use_default_provider_discovery() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture
            .detector()
            .detect(&[fixture.process(Some(Vec::new()))])
            .unwrap(),
        DetectionResult::Unchanged
    );
    assert_eq!(fixture.scans.load(Ordering::SeqCst), 0);
}

#[test]
fn unavailable_owned_association_does_not_use_default_provider_discovery() {
    let fixture = Fixture::new();
    let unavailable = fixture.root.join(UNAVAILABLE_EVIDENCE_FILE);
    assert_eq!(
        fixture
            .detector()
            .detect(&[fixture.process(Some(vec![unavailable]))])
            .unwrap(),
        DetectionResult::Unchanged
    );
    assert_eq!(fixture.scans.load(Ordering::SeqCst), 0);
}

#[test]
fn provider_foreign_associated_path_is_rejected_before_reading() {
    let fixture = Fixture::new();
    let foreign = fixture.shell_cwd.join("foreign.jsonl");
    fs::write(&foreign, FIRST_SESSION).unwrap();
    assert_eq!(
        fixture
            .detector()
            .detect(&[fixture.process(Some(vec![fixture.second.clone(), foreign]))])
            .unwrap(),
        DetectionResult::Unchanged
    );
    assert_eq!(fixture.inspections.load(Ordering::SeqCst), 0);
}
