use super::*;
use serde_json::{Value, json};
use std::fs;
use tempfile::tempdir;

const SESSION_ID: &str = "998259a8-f779-4505-b3aa-0d71ba2fbf43";
const OTHER_SESSION_ID: &str = "42dd54a1-d72b-4da1-b08e-ec73fe898751";
const INITIAL_DIRECTORY: &str = "project";
const CURRENT_DIRECTORY: &str = "sources";
const DATA_HOME_DIRECTORY: &str = "claude-home";
const PARENT_DIRECTORY: &str = "..";
const JSON_LINE_ENDING: &str = "\n";
const FILLER_KEY: &str = "filler";
const FILLER_VALUE: char = 'x';

#[test]
fn ordered_cwd_transition_selects_latest_observed_directory() {
    let fixture = Fixture::new();
    fixture.write(&[
        session_record(constants::MODE_RECORD, None),
        session_record(constants::USER_RECORD, Some(&fixture.initial_cwd)),
        session_record(constants::ASSISTANT_RECORD, Some(&fixture.current_cwd)),
    ]);
    let found = ClaudeAdapter
        .inspect_evidence(&fixture.evidence, &fixture.current_cwd)
        .unwrap()
        .unwrap();
    assert_eq!(found.cwd, fixture.current_cwd);
    assert_eq!(
        ClaudeAdapter
            .discover_evidence(&fixture.context(), &fixture.current_cwd)
            .unwrap(),
        [fixture.evidence.clone()]
    );
    assert!(
        ClaudeAdapter
            .inspect_evidence(&fixture.evidence, &fixture.initial_cwd)
            .unwrap()
            .is_none()
    );
}

#[test]
fn bounded_tail_transition_updates_the_head_anchored_cwd() {
    let fixture = Fixture::new();
    let filler_bytes =
        crate::agent::constants::EVIDENCE_HEAD_BYTES + crate::agent::constants::EVIDENCE_TAIL_BYTES;
    let filler = FILLER_VALUE.to_string().repeat(filler_bytes as usize);
    fixture.write(&[
        session_record(constants::USER_RECORD, Some(&fixture.initial_cwd)),
        json!({(FILLER_KEY): filler}),
        session_record(constants::USER_RECORD, Some(&fixture.current_cwd)),
    ]);
    let found = ClaudeAdapter
        .inspect_evidence(&fixture.evidence, &fixture.current_cwd)
        .unwrap()
        .unwrap();
    assert_eq!(found.cwd, fixture.current_cwd);
}

#[test]
fn conflicting_tail_session_fails_closed() {
    let fixture = Fixture::new();
    let filler_bytes =
        crate::agent::constants::EVIDENCE_HEAD_BYTES + crate::agent::constants::EVIDENCE_TAIL_BYTES;
    let filler = FILLER_VALUE.to_string().repeat(filler_bytes as usize);
    let mut conflict = session_record(constants::USER_RECORD, Some(&fixture.current_cwd));
    conflict[constants::SESSION_ID_KEY] = json!(OTHER_SESSION_ID);
    fixture.write(&[
        session_record(constants::USER_RECORD, Some(&fixture.initial_cwd)),
        json!({(FILLER_KEY): filler}),
        conflict,
    ]);
    assert!(
        ClaudeAdapter
            .inspect_evidence(&fixture.evidence, &fixture.current_cwd)
            .unwrap()
            .is_none()
    );
}

#[test]
fn unsafe_or_unbound_cwd_transition_fails_closed() {
    let fixture = Fixture::new();
    let unsafe_cwd = fixture
        .initial_cwd
        .join(CURRENT_DIRECTORY)
        .join(PARENT_DIRECTORY)
        .join(INITIAL_DIRECTORY);
    let invalid = [
        session_record(constants::USER_RECORD, Some(&unsafe_cwd)),
        json!({(constants::CWD_KEY): fixture.current_cwd}),
        session_record(constants::USER_RECORD, Some(Path::new(CURRENT_DIRECTORY))),
    ];
    for record in invalid {
        fixture.write(&[
            session_record(constants::USER_RECORD, Some(&fixture.current_cwd)),
            record,
        ]);
        assert!(
            ClaudeAdapter
                .inspect_evidence(&fixture.evidence, &fixture.current_cwd)
                .unwrap()
                .is_none()
        );
    }
}

#[cfg(unix)]
#[test]
fn symlink_cwd_matches_the_observed_canonical_directory() {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new();
    let alias = fixture
        .initial_cwd
        .parent()
        .unwrap()
        .join(CURRENT_DIRECTORY);
    symlink(&fixture.current_cwd, &alias).unwrap();
    fixture.write(&[
        session_record(constants::MODE_RECORD, None),
        session_record(constants::USER_RECORD, Some(&alias)),
    ]);
    let found = ClaudeAdapter
        .inspect_evidence(&fixture.evidence, &fixture.current_cwd)
        .unwrap()
        .unwrap();
    assert_eq!(found.cwd, fixture.current_cwd);
}

struct Fixture {
    _directory: tempfile::TempDir,
    initial_cwd: PathBuf,
    current_cwd: PathBuf,
    home: PathBuf,
    evidence: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempdir().unwrap();
        let initial_cwd = directory.path().join(INITIAL_DIRECTORY);
        let current_cwd = initial_cwd.join(CURRENT_DIRECTORY);
        let home = directory.path().join(DATA_HOME_DIRECTORY);
        let root = home.join(constants::EVIDENCE_DIR);
        let evidence_directory = discovery::project_directory(&root, &initial_cwd);
        fs::create_dir_all(&current_cwd).unwrap();
        fs::create_dir_all(&evidence_directory).unwrap();
        let evidence = evidence_directory.join(format!(
            "{SESSION_ID}.{}",
            crate::agent::constants::JSONL_EXTENSION
        ));
        Self {
            _directory: directory,
            initial_cwd,
            current_cwd,
            home,
            evidence,
        }
    }

    fn context(&self) -> DetectionContext {
        DetectionContext::new(
            uuid::Uuid::new_v4(),
            self.current_cwd.clone(),
            [(AgentKind::ClaudeCode, self.home.clone())],
        )
    }

    fn write(&self, values: &[Value]) {
        let mut contents = values
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join(JSON_LINE_ENDING);
        contents.push_str(JSON_LINE_ENDING);
        fs::write(&self.evidence, contents).unwrap();
    }
}

fn session_record(record_type: &str, cwd: Option<&Path>) -> Value {
    let mut record = json!({
        (constants::RECORD_TYPE_KEY): record_type,
        (constants::SESSION_ID_KEY): SESSION_ID,
    });
    if let Some(cwd) = cwd {
        record[constants::CWD_KEY] = json!(cwd);
    }
    record
}
