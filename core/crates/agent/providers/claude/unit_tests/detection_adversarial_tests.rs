use super::*;
use crate::agent::{AgentDetector, AgentRecoveryAvailability, AgentRegistry, DetectionResult};
use serde_json::json;
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

const OLD_SESSION_ID: &str = "06fd17d5-13c2-4a74-b190-b33e760876af";
const NEW_SESSION_ID: &str = "55212187-013a-4f59-a9e2-b6bc7dd2c94e";
const OTHER_SESSION_ID: &str = "c69caeac-a63a-4835-b690-6af53078d314";
const PROJECT_DIRECTORY: &str = "project";
const DATA_HOME_DIRECTORY: &str = "claude-home";
const LEFT_PROCESS_PID: u32 = 41_101;
const RIGHT_PROCESS_PID: u32 = 41_102;
const PROCESS_GROUP: i32 = 41_103;
const JSON_LINE_ENDING: char = '\n';

struct Fixture {
    _directory: tempfile::TempDir,
    cwd: PathBuf,
    home: PathBuf,
    evidence_root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().unwrap();
        let cwd = directory.path().join(PROJECT_DIRECTORY);
        let home = directory.path().join(DATA_HOME_DIRECTORY);
        let root = home.join(constants::EVIDENCE_DIR);
        let evidence_root = discovery::project_directory(&root, &cwd);
        fs::create_dir_all(&cwd).unwrap();
        fs::create_dir_all(&evidence_root).unwrap();
        Self {
            _directory: directory,
            cwd,
            home,
            evidence_root,
        }
    }

    fn evidence(&self, session_id: &str) -> PathBuf {
        self.evidence_root.join(format!(
            "{session_id}.{}",
            crate::agent::constants::JSONL_EXTENSION
        ))
    }

    fn write(&self, session_id: &str) {
        let record = json!({
            (constants::RECORD_TYPE_KEY): constants::USER_RECORD,
            (constants::SESSION_ID_KEY): session_id,
            (constants::CWD_KEY): self.cwd,
        });
        fs::write(
            self.evidence(session_id),
            format!("{record}{JSON_LINE_ENDING}"),
        )
        .unwrap();
    }

    fn detector(&self) -> AgentDetector {
        let context = DetectionContext::new(
            Uuid::new_v4(),
            self.cwd.clone(),
            [(AgentKind::ClaudeCode, self.home.clone())],
        );
        AgentDetector::new(context, yolo_agent_providers::builtin_agent_registry(), None).unwrap()
    }

    fn process(&self, pid: u32, session_id: &str, associated: Option<&str>) -> ObservedProcess {
        ObservedProcess {
            pid,
            start_time: u64::from(pid),
            parent_pid: crate::agent::constants::MIN_PROCESS_PID,
            process_group: PROCESS_GROUP,
            foreground_group: PROCESS_GROUP,
            executable: constants::EXECUTABLE.into(),
            argv: vec![
                constants::EXECUTABLE.into(),
                constants::RESUME_ARGUMENT.into(),
                session_id.into(),
            ],
            environment: None,
            cwd: Some(self.cwd.clone()),
            evidence_paths: associated.map(|id| vec![self.evidence(id)]),
        }
    }

    fn bind(&self, pid: u32, session_id: &str) -> AgentDetector {
        let mut detector = self.detector();
        let process = self.process(pid, session_id, Some(session_id));
        assert!(matches!(
            detector.detect(&[process]).unwrap(),
            DetectionResult::BindingChanged(binding)
                if binding.provider_session_id == session_id
                    && binding.state == AgentState::Unknown
        ));
        detector
    }
}

#[test]
fn same_cwd_terminals_ignore_each_others_interleaved_transcripts() {
    let fixture = Fixture::new();
    fixture.write(OLD_SESSION_ID);
    fixture.write(OTHER_SESSION_ID);
    let mut left = fixture.bind(LEFT_PROCESS_PID, OLD_SESSION_ID);
    let mut right = fixture.bind(RIGHT_PROCESS_PID, OTHER_SESSION_ID);

    fixture.write(NEW_SESSION_ID);
    fixture.write(OTHER_SESSION_ID);
    fixture.write(OLD_SESSION_ID);
    let right_process = fixture.process(RIGHT_PROCESS_PID, OTHER_SESSION_ID, None);
    let left_process = fixture.process(LEFT_PROCESS_PID, OLD_SESSION_ID, None);
    assert_eq!(
        right.detect(&[right_process]).unwrap(),
        DetectionResult::Unchanged
    );
    assert_eq!(
        left.detect(&[left_process]).unwrap(),
        DetectionResult::Unchanged
    );
    assert_session(left.session_exited(), OLD_SESSION_ID);
    assert_session(right.session_exited(), OTHER_SESSION_ID);
}

#[test]
fn rapid_clear_requires_association_before_recording_the_new_generation() {
    let fixture = Fixture::new();
    fixture.write(OLD_SESSION_ID);
    let mut detector = fixture.bind(LEFT_PROCESS_PID, OLD_SESSION_ID);
    fixture.write(NEW_SESSION_ID);

    let unassociated = fixture.process(LEFT_PROCESS_PID, OLD_SESSION_ID, None);
    assert_eq!(
        detector.detect(&[unassociated]).unwrap(),
        DetectionResult::Unchanged
    );
    let associated = fixture.process(LEFT_PROCESS_PID, OLD_SESSION_ID, Some(NEW_SESSION_ID));
    assert!(matches!(
        detector.detect(&[associated]).unwrap(),
        DetectionResult::BindingChanged(binding)
            if binding.provider_session_id == NEW_SESSION_ID
                && binding.state == AgentState::Unknown
    ));
}

fn assert_session(result: DetectionResult, expected: &str) {
    assert!(matches!(
        result,
        DetectionResult::StatusChanged(binding)
            if binding.provider_session_id == expected
                && binding.state == AgentState::Unknown
                && binding.recovery_availability == AgentRecoveryAvailability::Unavailable
    ));
}
