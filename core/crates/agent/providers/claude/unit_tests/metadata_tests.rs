use super::*;
use crate::claude::metadata::display_title;
use serde_json::{Value, json};
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

const SESSION_ID: &str = "025dd66f-2e68-448c-bec8-b035b5aeb30f";
const OTHER_SESSION_ID: &str = "5a47b9d3-69d2-4a15-8e49-26e513501a53";
const PROJECT_DIRECTORY: &str = "project";
const DATA_HOME_DIRECTORY: &str = "claude-home";
const SUBAGENT_DIRECTORY: &str = "subagents";
const JSON_LINE_ENDING: &str = "\n";
const FILLER: char = 'x';
const FILLER_KEY: &str = "filler";

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
        let evidence = transcript_path(&project, SESSION_ID);
        Self {
            _directory: directory,
            cwd,
            home,
            evidence,
        }
    }

    fn write(&self, values: &[Value]) {
        let contents = values
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join(JSON_LINE_ENDING);
        fs::write(&self.evidence, format!("{contents}{JSON_LINE_ENDING}")).unwrap();
    }

    fn context(&self) -> DetectionContext {
        DetectionContext::new(
            Uuid::new_v4(),
            self.cwd.clone(),
            [(AgentKind::ClaudeCode, self.home.clone())],
        )
    }
}

#[test]
fn leading_mode_and_ai_title_records_find_later_complete_metadata() {
    let fixture = Fixture::new();
    for (leading, complete) in [
        (constants::MODE_RECORD, constants::SYSTEM_RECORD),
        (constants::AI_TITLE_RECORD, constants::USER_RECORD),
    ] {
        fixture.write(&[
            json!({
                (constants::RECORD_TYPE_KEY): leading,
                (constants::SESSION_ID_KEY): SESSION_ID,
            }),
            json!({
                (constants::RECORD_TYPE_KEY): complete,
                (constants::SESSION_ID_KEY): SESSION_ID,
                (constants::CWD_KEY): fixture.cwd,
            }),
        ]);
        let found = ClaudeAdapter
            .inspect_evidence(&fixture.evidence, &fixture.cwd)
            .unwrap()
            .unwrap();
        assert_eq!(found.provider_session_id, SESSION_ID);
        assert_eq!(found.cwd, fixture.cwd);
    }
}

#[test]
fn conflicting_session_or_nonmatching_cwd_fails_closed() {
    let fixture = Fixture::new();
    let other_cwd = fixture.cwd.join(PROJECT_DIRECTORY);
    let initial = json!({
        (constants::RECORD_TYPE_KEY): constants::USER_RECORD,
        (constants::SESSION_ID_KEY): SESSION_ID,
        (constants::CWD_KEY): fixture.cwd,
    });
    fixture.write(&[
        initial,
        json!({(constants::SESSION_ID_KEY): OTHER_SESSION_ID}),
    ]);
    assert!(
        ClaudeAdapter
            .inspect_evidence(&fixture.evidence, &fixture.cwd)
            .unwrap()
            .is_none()
    );

    fixture.write(&[json!({
        (constants::SESSION_ID_KEY): SESSION_ID,
        (constants::CWD_KEY): other_cwd,
    })]);
    assert!(
        ClaudeAdapter
            .inspect_evidence(&fixture.evidence, &fixture.cwd)
            .unwrap()
            .is_none()
    );
}

#[test]
fn metadata_after_head_budget_fails_closed() {
    let fixture = Fixture::new();
    let filler = FILLER
        .to_string()
        .repeat(crate::agent::constants::EVIDENCE_HEAD_BYTES as usize);
    let leading = json!({
        (constants::RECORD_TYPE_KEY): constants::MODE_RECORD,
        (constants::SESSION_ID_KEY): SESSION_ID,
    });
    let complete = json!({
        (constants::RECORD_TYPE_KEY): constants::USER_RECORD,
        (constants::SESSION_ID_KEY): SESSION_ID,
        (constants::CWD_KEY): fixture.cwd,
    });
    fixture.write(&[leading, json!({(FILLER_KEY): filler}), complete]);
    assert!(
        ClaudeAdapter
            .inspect_evidence(&fixture.evidence, &fixture.cwd)
            .unwrap()
            .is_none()
    );
}

#[test]
fn filename_identity_and_subagent_coexistence_are_enforced() {
    let fixture = Fixture::new();
    fixture.write(&[json!({
        (constants::RECORD_TYPE_KEY): constants::USER_RECORD,
        (constants::SESSION_ID_KEY): SESSION_ID,
        (constants::CWD_KEY): fixture.cwd,
    })]);
    let project = fixture.evidence.parent().unwrap();
    let mismatched = transcript_path(project, OTHER_SESSION_ID);
    fs::copy(&fixture.evidence, &mismatched).unwrap();
    let subagent = project.join(SUBAGENT_DIRECTORY).join(format!(
        "{}{}.{}",
        constants::SUBAGENT_TRANSCRIPT_PREFIX,
        SESSION_ID,
        crate::agent::constants::JSONL_EXTENSION
    ));
    fs::create_dir_all(subagent.parent().unwrap()).unwrap();
    fs::copy(&fixture.evidence, &subagent).unwrap();

    assert!(
        ClaudeAdapter
            .inspect_evidence(&mismatched, &fixture.cwd)
            .unwrap()
            .is_none()
    );
    fs::remove_file(&mismatched).unwrap();
    assert!(
        ClaudeAdapter
            .inspect_evidence(&subagent, &fixture.cwd)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        ClaudeAdapter.evidence_files(&fixture.context()).unwrap(),
        [fixture.evidence.clone()]
    );
}

fn transcript_path(directory: &Path, session_id: &str) -> PathBuf {
    directory.join(format!(
        "{session_id}.{}",
        crate::agent::constants::JSONL_EXTENSION
    ))
}


include!("metadata_title_selection.rs");
