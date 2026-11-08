use super::super::constants;
use std::collections::BTreeSet;

use super::fixture;

#[test]
fn claude_2_1_222_fixture_preserves_redacted_real_schema() {
    let records = fixture::records();
    assert_eq!(records.len(), fixture::RECORD_COUNT);
    let types = records
        .iter()
        .filter_map(|record| record.get(constants::RECORD_TYPE_KEY)?.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        types,
        BTreeSet::from([
            constants::AI_TITLE_RECORD,
            constants::ASSISTANT_RECORD,
            constants::MODE_RECORD,
            constants::PERMISSION_MODE_RECORD,
            constants::SYSTEM_RECORD,
            constants::USER_RECORD,
        ])
    );
    let content_types = records
        .iter()
        .flat_map(fixture::content_blocks)
        .filter_map(|block| block.get(constants::CONTENT_TYPE_KEY)?.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        content_types,
        BTreeSet::from([
            constants::TEXT_CONTENT,
            constants::THINKING_CONTENT,
            constants::TOOL_RESULT_CONTENT,
            constants::TOOL_USE_CONTENT,
        ])
    );
    let prompt_content = records[fixture::USER_PROMPT_INDEX]
        .get(constants::MESSAGE_KEY)
        .and_then(|message| message.get(constants::CONTENT_KEY));
    assert!(prompt_content.is_some_and(serde_json::Value::is_string));
    assert!(records[fixture::MODE_INDEX][fixture::MODE_KEY].is_string());
    assert!(records[fixture::PERMISSION_MODE_INDEX][fixture::PERMISSION_MODE_KEY].is_string());
    assert!(records[fixture::AI_TITLE_INDEX][fixture::AI_TITLE_KEY].is_string());
    for record in &records {
        if let Some(version) = record.get(fixture::VERSION_KEY) {
            assert_eq!(version.as_str(), Some(fixture::CLAUDE_VERSION));
        }
        if let Some(session_id) = record.get(constants::SESSION_ID_KEY) {
            assert_eq!(session_id.as_str(), Some(fixture::SESSION_ID));
        }
        if let Some(cwd) = record.get(constants::CWD_KEY) {
            assert_eq!(cwd.as_str(), Some(fixture::CWD));
        }
    }
    let bytes = std::str::from_utf8(fixture::FIXTURE_BYTES).unwrap();
    assert!(!bytes.contains(fixture::FORBIDDEN_HOME_PREFIX));
    assert!(bytes.contains(fixture::REDACTED_MARKER));
}

#[test]
fn fixture_keeps_observed_additional_fields_permissive() {
    let records = fixture::records();
    let prompt = &records[fixture::USER_PROMPT_INDEX];
    assert!(prompt[fixture::THINKING_METADATA_KEY].is_object());
    assert!(prompt[fixture::TODOS_KEY].is_array());
    assert!(prompt[fixture::PROMPT_ID_KEY].is_string());
    let tool_turn = &records[fixture::ASSISTANT_TOOL_USE_INDEX];
    assert!(tool_turn[fixture::EFFORT_KEY].is_string());
    assert!(tool_turn[fixture::REQUEST_ID_KEY].is_string());
    let message = &tool_turn[constants::MESSAGE_KEY];
    assert!(message[constants::CONTEXT_MANAGEMENT_KEY].is_object());
    let blocks = fixture::content_blocks(tool_turn);
    assert!(blocks[fixture::THINKING_BLOCK_INDEX][fixture::SIGNATURE_KEY].is_string());
    assert!(blocks[fixture::TOOL_USE_BLOCK_INDEX][fixture::CALLER_KEY].is_object());
    let result = &records[fixture::USER_TOOL_RESULT_INDEX];
    assert!(result[fixture::TOOL_USE_RESULT_KEY].is_object());
    assert!(result[fixture::SOURCE_TOOL_UUID_KEY].is_string());
    let duration = &records[fixture::TURN_DURATION_INDEX];
    assert!(duration[fixture::PENDING_AGENT_COUNT_KEY].is_number());
}
