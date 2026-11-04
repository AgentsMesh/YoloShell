use super::*;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use uuid::Uuid;
const PROCESS_PID: u32 = 404;
const PROCESS_GROUP: i32 = 81;
const OLD_SESSION_ID: &str = "old-session";
const NEW_SESSION_ID: &str = "new-session";
const OTHER_SESSION_ID: &str = "other-session";
struct EvidenceAdapter {
    paths: Vec<PathBuf>,
}
impl AgentAdapter for EvidenceAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }
    fn matches_process(&self, process: &ObservedProcess) -> bool {
        process.executable == crate::agent::codex::constants::EXECUTABLE
    }
    fn evidence_discovery_policy(&self) -> EvidenceDiscoveryPolicy {
        EvidenceDiscoveryPolicy::ScopedStrongHint
    }
    fn process_session_id(&self, process: &ObservedProcess) -> Option<String> {
        process
            .argv
            .get(1)
            .and_then(|_| process.argv.last())
            .cloned()
    }
    fn evidence_files(&self, _: &DetectionContext) -> Result<Vec<PathBuf>> {
        Ok(self
            .paths
            .iter()
            .filter(|path| path.is_file())
            .cloned()
            .collect())
    }
    fn inspect_evidence(&self, path: &Path, cwd: &Path) -> Result<Option<AgentEvidence>> {
        let session_id = fs::read_to_string(path)?;
        Ok(Some(AgentEvidence {
            provider_session_id: session_id,
            cwd: cwd.into(),
            path: path.into(),
            data_home: path.parent().unwrap().into(),
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

#[test]
fn exact_identity_ignores_other_evidence_in_the_same_cwd() {
    let fixture = Fixture::new(&[OLD_SESSION_ID, NEW_SESSION_ID]);
    let mut detector = fixture.detector();
    fixture.write(NEW_SESSION_ID);
    assert!(matches!(
        detector.detect(&[fixture.process(NEW_SESSION_ID)]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == NEW_SESSION_ID
    ));
}

#[test]
fn exact_identity_is_stable_when_concurrent_evidence_order_reverses() {
    let fixture = Fixture::new(&[NEW_SESSION_ID, OTHER_SESSION_ID]);
    let mut detector = fixture.detector();
    fixture.write(NEW_SESSION_ID);
    fixture.write(OTHER_SESSION_ID);
    assert!(matches!(
        detector.detect(&[fixture.process(NEW_SESSION_ID)]).unwrap(),
        DetectionResult::BindingChanged(binding) if binding.provider_session_id == NEW_SESSION_ID
    ));
}

#[test]
fn evidence_created_after_the_first_process_scan_is_detected() {
    let fixture = Fixture::new(&[NEW_SESSION_ID]);
    let mut detector = fixture.detector();
    let process = fixture.process(NEW_SESSION_ID);
    assert_eq!(
        detector.detect(std::slice::from_ref(&process)).unwrap(),
        DetectionResult::Unchanged
    );
    fixture.write(NEW_SESSION_ID);
    assert!(matches!(
        detector.detect(&[process]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == NEW_SESSION_ID
    ));
}

#[test]
fn a_second_process_uses_new_evidence_instead_of_the_previous_binding() {
    let fixture = Fixture::new(&[NEW_SESSION_ID, OTHER_SESSION_ID]);
    let mut detector = fixture.detector();
    fixture.write(NEW_SESSION_ID);
    assert!(matches!(
        detector.detect(&[fixture.process(NEW_SESSION_ID)]).unwrap(),
        DetectionResult::BindingChanged(_)
    ));
    fixture.write(OTHER_SESSION_ID);
    let mut second = fixture.process(OTHER_SESSION_ID);
    second.pid += 1;
    assert!(matches!(
        detector.detect(&[second]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == OTHER_SESSION_ID
    ));
}

struct Fixture {
    _directory: tempfile::TempDir,
    cwd: PathBuf,
    paths: Vec<PathBuf>,
}

impl Fixture {
    fn new(ids: &[&str]) -> Self {
        let directory = tempdir().unwrap();
        let cwd = directory.path().join("project");
        fs::create_dir_all(&cwd).unwrap();
        let paths: Vec<PathBuf> = ids
            .iter()
            .map(|id| directory.path().join(format!("{id}.jsonl")))
            .collect();
        if ids.contains(&OLD_SESSION_ID) {
            fs::write(&paths[0], OLD_SESSION_ID).unwrap();
        }
        Self {
            _directory: directory,
            cwd,
            paths,
        }
    }

    fn detector(&self) -> AgentDetector {
        let registry = AgentRegistry::from_adapters(vec![Box::new(EvidenceAdapter {
            paths: self.paths.clone(),
        })])
        .unwrap();
        AgentDetector::new(
            DetectionContext::new(
                Uuid::new_v4(),
                self.cwd.clone(),
                [(AgentKind::Codex, self.cwd.clone())],
            ),
            registry,
            None,
        )
        .unwrap()
    }

    fn write(&self, id: &str) {
        let path = self
            .paths
            .iter()
            .find(|path| path.file_stem().is_some_and(|stem| stem == id))
            .unwrap();
        fs::write(path, id).unwrap();
    }

    fn process(&self, session_id: &str) -> ObservedProcess {
        ObservedProcess {
            pid: PROCESS_PID,
            start_time: u64::from(PROCESS_PID),
            parent_pid: 1,
            process_group: PROCESS_GROUP,
            foreground_group: PROCESS_GROUP,
            executable: crate::agent::codex::constants::EXECUTABLE.into(),
            argv: vec![
                crate::agent::codex::constants::EXECUTABLE.into(),
                session_id.into(),
            ],
            environment: None,
            cwd: Some(self.cwd.clone()),
            evidence_paths: None,
        }
    }
}
