use super::RolloutMetadata;
use crate::codex::{constants, test_support};
use serde_json::{Value, json};

const TIMESTAMP_KEY: &str = "timestamp";
const TIMESTAMP: &str = "2026-08-05T12:00:00.000Z";
const ORIGINATOR_KEY: &str = "originator";
const CLI_VERSION: &str = "0.146.0";
const NESTED_THREAD_SOURCE: &str = "subagent";
const MODEL_PROVIDER: &str = "openai";
const TEXT_KEY: &str = "text";
const SANITIZED_INSTRUCTIONS: &str = "sanitized";
const HISTORY_MODE: &str = "legacy";
const DEPTH_KEY: &str = "depth";
const NESTED_DEPTH: i32 = 1;
const FIRST_AGENT_PATH: &str = "/root/reviewer";
const SECOND_AGENT_PATH: &str = "/root/worker";
const FIRST_AGENT_NAME: &str = "reviewer";
const SECOND_AGENT_NAME: &str = "worker";
const AGENT_ROLE: &str = "reviewer";
const REVIEW_SOURCE: &str = "review";
const DAEMON_SOURCE: &str = "vscode";
const TUI_ORIGINATOR: &str = "codex-tui";

#[test]
fn a_daemon_created_user_thread_can_be_resumed_by_the_terminal() {
    let fixture = test_support::Fixture::new();
    let path = fixture.evidence(test_support::SESSION_ID);
    let mut record = base_metadata(
        &fixture, test_support::SESSION_ID, json!(DAEMON_SOURCE), constants::USER_THREAD_SOURCE,
    );
    record[constants::PAYLOAD_KEY][ORIGINATOR_KEY] = json!(TUI_ORIGINATOR);
    assert!(RolloutMetadata::from_head(&path, &[record.clone()], &fixture.cwd).is_some());

    record[constants::PAYLOAD_KEY][constants::THREAD_SOURCE_KEY] = json!(NESTED_THREAD_SOURCE);
    assert!(RolloutMetadata::from_head(&path, &[record.clone()], &fixture.cwd).is_none());
    record[constants::PAYLOAD_KEY][constants::SOURCE_KEY] = json!(constants::CLI_SOURCE);
    record[constants::PAYLOAD_KEY][constants::THREAD_SOURCE_KEY] = json!({});
    assert!(RolloutMetadata::from_head(&path, &[record], &fixture.cwd).is_none());
}

#[test]
fn real_0146_head_selects_cli_among_thread_spawn_sessions() {
    let fixture = test_support::Fixture::new();
    let path = fixture.evidence(test_support::SESSION_ID);
    let records = [
        nested_metadata(
            &fixture,
            test_support::OTHER_SESSION_ID,
            FIRST_AGENT_PATH,
            FIRST_AGENT_NAME,
            true,
        ),
        root_metadata(&fixture),
        nested_metadata(
            &fixture,
            test_support::NEW_SESSION_ID,
            SECOND_AGENT_PATH,
            SECOND_AGENT_NAME,
            false,
        ),
    ];

    let found = RolloutMetadata::from_head(&path, &records, &fixture.cwd).unwrap();
    assert_eq!(found.session_id, test_support::SESSION_ID);
    assert_eq!(found.cwd, fixture.cwd.canonicalize().unwrap());
}

#[test]
fn unit_subagent_sources_do_not_invalidate_the_cli_identity() {
    let fixture = test_support::Fixture::new();
    let path = fixture.evidence(test_support::SESSION_ID);
    let nested = base_metadata(
        &fixture,
        test_support::OTHER_SESSION_ID,
        json!({(constants::SUBAGENT_SOURCE_KEY): REVIEW_SOURCE}),
        NESTED_THREAD_SOURCE,
    );
    assert!(
        RolloutMetadata::from_head(&path, &[nested, root_metadata(&fixture)], &fixture.cwd)
            .is_some()
    );
}

fn root_metadata(fixture: &test_support::Fixture) -> Value {
    base_metadata(
        fixture,
        test_support::SESSION_ID,
        json!(constants::CLI_SOURCE),
        constants::USER_THREAD_SOURCE,
    )
}

fn nested_metadata(
    fixture: &test_support::Fixture,
    id: &str,
    agent_path: &str,
    agent_name: &str,
    forked: bool,
) -> Value {
    let source = json!({
        (constants::SUBAGENT_SOURCE_KEY): {
            (constants::REAL_THREAD_SPAWN_KEY): {
                (constants::REAL_PARENT_THREAD_ID_KEY): test_support::SESSION_ID,
                (DEPTH_KEY): NESTED_DEPTH,
                (constants::REAL_AGENT_PATH_KEY): agent_path,
                (constants::REAL_AGENT_NICKNAME_KEY): agent_name,
                (constants::REAL_AGENT_ROLE_KEY): AGENT_ROLE,
            }
        }
    });
    let mut value = base_metadata(fixture, id, source, NESTED_THREAD_SOURCE);
    let payload = &mut value[constants::PAYLOAD_KEY];
    payload[constants::REAL_PARENT_THREAD_ID_KEY] = json!(test_support::SESSION_ID);
    payload[constants::REAL_AGENT_PATH_KEY] = json!(agent_path);
    payload[constants::REAL_AGENT_NICKNAME_KEY] = json!(agent_name);
    payload[constants::REAL_AGENT_ROLE_KEY] = json!(AGENT_ROLE);
    if forked {
        payload[constants::REAL_FORKED_FROM_ID_KEY] = json!(test_support::SESSION_ID);
    }
    value
}

fn base_metadata(
    fixture: &test_support::Fixture,
    id: &str,
    source: Value,
    thread_source: &str,
) -> Value {
    let mut value = fixture.metadata(id, source);
    let payload = &mut value[constants::PAYLOAD_KEY];
    payload[TIMESTAMP_KEY] = json!(TIMESTAMP);
    payload[ORIGINATOR_KEY] = json!(constants::REAL_ORIGINATOR);
    payload[constants::REAL_CLI_VERSION_KEY] = json!(CLI_VERSION);
    payload[constants::REAL_SESSION_LINEAGE_ID_KEY] = json!(test_support::SESSION_ID);
    payload[constants::THREAD_SOURCE_KEY] = json!(thread_source);
    payload[constants::REAL_MODEL_PROVIDER_KEY] = json!(MODEL_PROVIDER);
    payload[constants::REAL_BASE_INSTRUCTIONS_KEY] = json!({(TEXT_KEY): SANITIZED_INSTRUCTIONS});
    payload[constants::REAL_HISTORY_MODE_KEY] = json!(HISTORY_MODE);
    value
}
