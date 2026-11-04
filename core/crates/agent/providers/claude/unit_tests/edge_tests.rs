use super::*;
use serde_json::{Value, json};
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

const COMPLETION_TEXT: &str = "Done";
const SESSION_ID: &str = "03cb8465-df62-4ff1-a505-19f488f60f7f";
const OTHER_SESSION_ID: &str = "3fa85f64-5717-4562-b3fc-2c963f66afa6";

fn context(claude_home: PathBuf, expected_cwd: PathBuf) -> DetectionContext {
    DetectionContext::new(
        Uuid::new_v4(),
        expected_cwd,
        [(AgentKind::ClaudeCode, claude_home)],
    )
}

fn write_values(path: &Path, values: &[Value]) {
    let contents = values
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, format!("{contents}\n")).unwrap();
}

#[test]
fn discovers_nested_evidence_by_session_file() {
    let directory = tempdir().unwrap();
    let cwd = directory.path().join("project");
    let home = directory.path().join("home");
    let evidence_root = home.join(constants::EVIDENCE_DIR);
    let nested = discovery::project_directory(&evidence_root, &cwd);
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(&nested).unwrap();
    let evidence = nested.join(format!(
        "{SESSION_ID}.{}",
        crate::agent::constants::JSONL_EXTENSION
    ));
    let adapter = ClaudeAdapter;

    write_values(&evidence, &[json!({"type": "user"})]);
    assert_eq!(
        adapter.evidence_files(&context(home, cwd)).unwrap(),
        std::slice::from_ref(&evidence)
    );
    assert_eq!(adapter.kind(), AgentKind::ClaudeCode);
}

#[test]
fn rejects_invalid_metadata_and_derives_owned_data_home() {
    let directory = tempdir().unwrap();
    let cwd = directory.path().join("project");
    let other = directory.path().join("other");
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(&other).unwrap();
    let evidence_root = directory.path().join(constants::EVIDENCE_DIR);
    let evidence_directory = evidence_root.join("project-scope");
    fs::create_dir_all(&evidence_directory).unwrap();
    let evidence = evidence_directory.join(format!(
        "{SESSION_ID}.{}",
        crate::agent::constants::JSONL_EXTENSION
    ));
    let adapter = ClaudeAdapter;

    let rejected = [
        json!({"type": "user", "cwd": cwd}),
        json!({"type": "user", "sessionId": SESSION_ID}),
        json!({"type": "user", "sessionId": SESSION_ID, "cwd": other}),
    ];
    for value in rejected {
        write_values(&evidence, &[value]);
        assert!(adapter.inspect_evidence(&evidence, &cwd).unwrap().is_none());
    }
    write_values(
        &evidence,
        &[
            json!({"type": "user"}),
            json!({"type": "user", "sessionId": OTHER_SESSION_ID, "cwd": cwd}),
        ],
    );
    assert!(adapter.inspect_evidence(&evidence, &cwd).unwrap().is_none());

    write_values(
        &evidence,
        &[json!({"type": "user", "sessionId": SESSION_ID, "cwd": cwd})],
    );
    let found = adapter.inspect_evidence(&evidence, &cwd).unwrap().unwrap();
    assert_eq!(found.data_home, directory.path());
    assert_eq!(found.state, AgentState::Unknown);
}

fn completed_assistant() -> Value {
    json!({
        (constants::RECORD_TYPE_KEY): constants::ASSISTANT_RECORD,
        (constants::MESSAGE_KEY): {
            (constants::CONTENT_KEY): [{
                (constants::CONTENT_TYPE_KEY): constants::TEXT_CONTENT,
                (constants::TEXT_VALUE_KEY): COMPLETION_TEXT,
            }],
            (constants::STOP_REASON_KEY): constants::END_TURN_REASON,
        },
    })
}
