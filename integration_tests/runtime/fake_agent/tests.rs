use super::*;
use serde_json::{Value, json};
use std::io::Write;
use std::path::Path;
use yolo_runtime::agent::{AgentAdapter, claude, codex};

const TEST_SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const CODEX_RECORD_COUNT: usize = 2;
const CLAUDE_RECORD_COUNT: usize = 1;
const METADATA_RECORD_INDEX: usize = 0;
const STARTED_RECORD_INDEX: usize = 1;
const CLAUDE_COMPLETION_TEXT: &str = "Done";

#[test]
fn evidence_writer_remains_open_after_initial_records_are_flushed() {
    let directory = tempfile::tempdir().unwrap();
    let mut writer =
        write_evidence(FakeAgentProvider::Codex, directory.path(), TEST_SESSION_ID).unwrap();
    let path = FakeAgentProvider::Codex.evidence_path(directory.path(), TEST_SESSION_ID);

    let initial = evidence_values(&path);
    assert_eq!(initial.len(), CODEX_RECORD_COUNT);
    assert_eq!(
        initial[METADATA_RECORD_INDEX][codex::constants::RECORD_TYPE_KEY],
        codex::constants::SESSION_META_TYPE
    );
    assert_eq!(
        initial[METADATA_RECORD_INDEX][codex::constants::PAYLOAD_KEY]
            [codex::constants::SESSION_ID_KEY],
        TEST_SESSION_ID
    );
    assert_eq!(
        initial[METADATA_RECORD_INDEX][codex::constants::PAYLOAD_KEY][codex::constants::SOURCE_KEY],
        codex::constants::CLI_SOURCE
    );
    assert_eq!(
        initial[METADATA_RECORD_INDEX][codex::constants::PAYLOAD_KEY][codex::constants::CWD_KEY],
        std::env::current_dir().unwrap().display().to_string()
    );
    assert_eq!(
        initial[STARTED_RECORD_INDEX][codex::constants::PAYLOAD_KEY]
            [codex::constants::EVENT_KIND_KEY],
        codex::constants::TASK_STARTED_EVENT
    );

    let completion = json!({
        (codex::constants::RECORD_TYPE_KEY): codex::constants::EVENT_MESSAGE_TYPE,
        (codex::constants::PAYLOAD_KEY): {
            (codex::constants::EVENT_KIND_KEY): codex::constants::TASK_COMPLETE_EVENT,
        },
    });
    serde_json::to_writer(&mut writer, &completion).unwrap();
    writeln!(writer).unwrap();
    writer.flush().unwrap();
    assert_eq!(
        evidence_values(&path).last().unwrap()[codex::constants::PAYLOAD_KEY]
            [codex::constants::EVENT_KIND_KEY],
        codex::constants::TASK_COMPLETE_EVENT
    );
}

#[test]
fn claude_evidence_and_provider_selection_use_claude_contracts() {
    let directory = tempfile::tempdir().unwrap();
    let mut writer =
        write_evidence(FakeAgentProvider::Claude, directory.path(), TEST_SESSION_ID).unwrap();
    let path = FakeAgentProvider::Claude.evidence_path(directory.path(), TEST_SESSION_ID);
    let values = evidence_values(&path);
    assert_eq!(values.len(), CLAUDE_RECORD_COUNT);
    assert_eq!(
        values[METADATA_RECORD_INDEX][claude::constants::RECORD_TYPE_KEY],
        claude::constants::USER_RECORD
    );
    assert_eq!(
        values[METADATA_RECORD_INDEX][claude::constants::SESSION_ID_KEY],
        TEST_SESSION_ID
    );
    assert_eq!(
        values[METADATA_RECORD_INDEX][claude::constants::CWD_KEY],
        std::env::current_dir().unwrap().display().to_string()
    );
    assert_eq!(
        FakeAgentProvider::from_executable(Path::new(claude::constants::EXECUTABLE)),
        Ok(FakeAgentProvider::Claude)
    );
    assert_eq!(
        FakeAgentProvider::from_executable(Path::new(codex::constants::EXECUTABLE)),
        Ok(FakeAgentProvider::Codex)
    );
    let completion = json!({
        (claude::constants::RECORD_TYPE_KEY): claude::constants::ASSISTANT_RECORD,
        (claude::constants::MESSAGE_KEY): {
            (claude::constants::CONTENT_KEY): [{
                (claude::constants::CONTENT_TYPE_KEY): claude::constants::TEXT_CONTENT,
                (claude::constants::TEXT_VALUE_KEY): CLAUDE_COMPLETION_TEXT,
            }],
            (claude::constants::STOP_REASON_KEY): claude::constants::END_TURN_REASON,
        },
    });
    serde_json::to_writer(&mut writer, &completion).unwrap();
    writeln!(writer).unwrap();
    writer.flush().unwrap();
    assert_provider_contracts();
}

fn assert_provider_contracts() {
    let cases = [
        (
            FakeAgentProvider::Codex,
            codex::constants::DATA_HOME_ENV,
            codex::constants::EVIDENCE_DIR,
            codex::constants::RESUME_ARGUMENT,
        ),
        (
            FakeAgentProvider::Claude,
            claude::constants::DATA_HOME_ENV,
            claude::constants::EVIDENCE_DIR,
            claude::constants::RESUME_ARGUMENT,
        ),
    ];
    for (provider, data_home, evidence, resume) in cases {
        assert_eq!(provider.data_home_environment(), data_home);
        assert_eq!(provider.evidence_directory(), evidence);
        assert_eq!(provider.resume_argument(), resume);
    }
}

fn evidence_values(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
