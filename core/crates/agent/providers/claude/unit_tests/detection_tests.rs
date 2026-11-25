use super::*;
use crate::agent::{AgentDetector, AgentRegistry, DetectionResult};
use serde_json::json;
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

const SESSION_ID: &str = "d573778a-306d-4386-a60d-12498116bc86";
const PROJECT_DIRECTORY: &str = "project";
const DATA_HOME_DIRECTORY: &str = "claude-home";
const FOREIGN_EVIDENCE_FILE: &str = "foreign.jsonl";
const INVALID_SESSION_ID: &str = "invalid-session";
const PROCESS_PID: u32 = 907;
const PROCESS_GROUP: i32 = 117;
const JSON_LINE_ENDING: char = '\n';

struct Fixture {
    _directory: tempfile::TempDir,
    cwd: PathBuf,
    home: PathBuf,
    evidence: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().unwrap();
        let cwd = directory.path().join(PROJECT_DIRECTORY);
        let home = directory.path().join(DATA_HOME_DIRECTORY);
        let root = home.join(constants::EVIDENCE_DIR);
        let project = discovery::project_directory(&root, &cwd);
        fs::create_dir_all(&cwd).unwrap();
        fs::create_dir_all(&project).unwrap();
        let evidence = project.join(format!(
            "{SESSION_ID}.{}",
            crate::agent::constants::JSONL_EXTENSION
        ));
        Self {
            _directory: directory,
            cwd,
            home,
            evidence,
        }
    }

    fn detector(&self) -> AgentDetector {
        AgentDetector::new(
            DetectionContext::new(
                Uuid::new_v4(),
                self.cwd.clone(),
                [(AgentKind::ClaudeCode, self.home.clone())],
            ),
            yolo_agent_providers::builtin_agent_registry(),
            None,
        )
        .unwrap()
    }

    fn write(&self) {
        let record = json!({
            (constants::RECORD_TYPE_KEY): constants::USER_RECORD,
            (constants::SESSION_ID_KEY): SESSION_ID,
            (constants::CWD_KEY): self.cwd,
        });
        fs::write(&self.evidence, format!("{record}{JSON_LINE_ENDING}")).unwrap();
    }

    fn process(&self, arguments: &[&str], evidence_paths: Option<Vec<PathBuf>>) -> ObservedProcess {
        ObservedProcess {
            pid: PROCESS_PID,
            start_time: u64::from(PROCESS_PID),
            parent_pid: crate::agent::constants::MIN_PROCESS_PID,
            process_group: PROCESS_GROUP,
            foreground_group: PROCESS_GROUP,
            executable: constants::EXECUTABLE.into(),
            argv: arguments
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect(),
            environment: None,
            cwd: Some(self.cwd.clone()),
            evidence_paths,
        }
    }
}

#[test]
fn bare_process_never_discovers_from_empty_or_unavailable_association() {
    let fixture = Fixture::new();
    let mut detector = fixture.detector();
    fixture.write();
    for paths in [Some(Vec::new()), None] {
        assert_eq!(
            detector
                .detect(&[fixture.process(&[constants::EXECUTABLE], paths)])
                .unwrap(),
            DetectionResult::Unchanged
        );
    }
}

#[test]
fn canonical_argv_identity_scopes_unassociated_discovery() {
    let fixture = Fixture::new();
    let mut detector = fixture.detector();
    fixture.write();
    let process = fixture.process(
        &[
            constants::EXECUTABLE,
            constants::RESUME_ARGUMENT,
            SESSION_ID,
        ],
        Some(Vec::new()),
    );
    assert!(matches!(
        detector.detect(&[process]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == SESSION_ID
                && binding.evidence_path == fixture.evidence.display().to_string()
                && binding.state == AgentState::Unknown
    ));
}

#[test]
fn malformed_argv_identity_fails_closed() {
    let fixture = Fixture::new();
    let mut detector = fixture.detector();
    fixture.write();
    let process = fixture.process(
        &[
            constants::EXECUTABLE,
            constants::SESSION_ID_ARGUMENT,
            INVALID_SESSION_ID,
        ],
        Some(Vec::new()),
    );
    assert_eq!(
        detector.detect(&[process]).unwrap(),
        DetectionResult::Unchanged
    );
}

#[test]
fn associated_transcript_is_authoritative_without_argv_identity() {
    let fixture = Fixture::new();
    let mut detector = fixture.detector();
    fixture.write();
    let process = fixture.process(
        &[constants::EXECUTABLE],
        Some(vec![fixture.evidence.clone()]),
    );
    assert!(matches!(
        detector.detect(&[process]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == SESSION_ID && binding.state == AgentState::Unknown
    ));
}

#[test]
fn foreign_association_cannot_bypass_into_provider_discovery() {
    let fixture = Fixture::new();
    let mut detector = fixture.detector();
    fixture.write();
    let foreign = fixture.cwd.join(FOREIGN_EVIDENCE_FILE);
    fs::write(&foreign, SESSION_ID).unwrap();
    let paths = Some(vec![fixture.evidence.clone(), foreign]);
    assert_eq!(
        detector
            .detect(&[fixture.process(&[constants::EXECUTABLE], paths)])
            .unwrap(),
        DetectionResult::Unchanged
    );
}
