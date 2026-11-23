use super::{RolloutMetadata, canonical_session_id, display_title, rollout_session_id};
use crate::codex::{constants, test_support};
use serde_json::{Value, json};
use std::path::Path;

const INVALID_SESSION_ID: &str = "not-a-uuid";
const COMPACT_SESSION_ID: &str = "550e8400e29b41d4a716446655440000";
const RELATIVE_CWD: &str = "relative/project";
const PARENT_CWD: &str = "/project/../other";
const WRONG_SOURCE: &str = "api";
const SUBAGENT_VALUE: &str = "thread";
const OTHER_CWD_DIRECTORY: &str = "other";

#[test]
fn only_lowercase_hyphenated_ids_and_rollout_names_are_canonical() {
    assert_eq!(
        canonical_session_id(test_support::SESSION_ID).as_deref(),
        Some(test_support::SESSION_ID)
    );
    for invalid in [
        INVALID_SESSION_ID,
        COMPACT_SESSION_ID,
        &test_support::SESSION_ID.to_uppercase(),
    ] {
        assert!(canonical_session_id(invalid).is_none());
    }

    let fixture = test_support::Fixture::new();
    let path = fixture.evidence(test_support::SESSION_ID);
    assert_eq!(rollout_session_id(&path), Some(test_support::SESSION_ID));
    let short_name = format!(
        "{}{}.{}",
        constants::ROLLOUT_FILE_PREFIX,
        test_support::SESSION_ID,
        crate::agent::constants::JSONL_EXTENSION,
    );
    assert!(rollout_session_id(Path::new(&short_name)).is_none());
    assert!(rollout_session_id(Path::new(INVALID_SESSION_ID)).is_none());
}

#[test]
fn main_metadata_requires_canonical_cli_records_with_matching_identity() {
    let fixture = test_support::Fixture::new();
    let path = fixture.evidence(test_support::SESSION_ID);
    let accepted = fixture.metadata(test_support::SESSION_ID, json!(constants::CLI_SOURCE));
    let found = RolloutMetadata::from_head(&path, &[accepted.clone()], &fixture.cwd).unwrap();
    assert_eq!(found.session_id, test_support::SESSION_ID);
    assert_eq!(found.cwd, fixture.cwd.canonicalize().unwrap());

    let mut missing_source = accepted.clone();
    missing_source[constants::PAYLOAD_KEY]
        .as_object_mut()
        .unwrap()
        .remove(constants::SOURCE_KEY);
    let rejected = [
        fixture.metadata(test_support::SESSION_ID, json!(WRONG_SOURCE)),
        fixture.metadata(
            test_support::SESSION_ID,
            json!({(constants::SUBAGENT_SOURCE_KEY): SUBAGENT_VALUE}),
        ),
        fixture.metadata(test_support::SESSION_ID, Value::Null),
        missing_source,
        fixture.metadata(test_support::OTHER_SESSION_ID, json!(constants::CLI_SOURCE)),
    ];
    for value in rejected {
        assert!(RolloutMetadata::from_head(&path, &[value], &fixture.cwd).is_none());
    }
}

#[test]
fn codex_exec_sessions_are_main_sessions_like_cli_sessions() {
    let fixture = test_support::Fixture::new();
    let path = fixture.evidence(test_support::SESSION_ID);
    let value = fixture.metadata(test_support::SESSION_ID, json!(constants::EXEC_SOURCE));
    let found = RolloutMetadata::from_head(&path, &[value], &fixture.cwd).unwrap();
    assert_eq!(found.session_id, test_support::SESSION_ID);
    assert_eq!(found.cwd, fixture.cwd.canonicalize().unwrap());
}

#[test]
fn repeated_cli_metadata_must_preserve_main_identity_and_canonical_cwd() {
    let fixture = test_support::Fixture::new();
    let path = fixture.evidence(test_support::SESSION_ID);
    let accepted = fixture.metadata(test_support::SESSION_ID, json!(constants::CLI_SOURCE));
    let records = [accepted.clone(), accepted.clone(), accepted.clone()];
    assert!(RolloutMetadata::from_head(&path, &records, &fixture.cwd).is_some());

    let other_cwd = fixture.cwd.with_file_name(OTHER_CWD_DIRECTORY);
    std::fs::create_dir(&other_cwd).unwrap();
    let mut changed_cwd = accepted.clone();
    changed_cwd[constants::PAYLOAD_KEY][constants::CWD_KEY] = json!(other_cwd);
    let conflicts = [
        fixture.metadata(test_support::OTHER_SESSION_ID, json!(constants::CLI_SOURCE)),
        changed_cwd,
        json!({
            (constants::RECORD_TYPE_KEY): constants::SESSION_META_TYPE,
            (constants::PAYLOAD_KEY): {
                (constants::SESSION_ID_KEY): test_support::SESSION_ID,
                (constants::SOURCE_KEY): constants::CLI_SOURCE,
            }
        }),
        json!({
            (constants::RECORD_TYPE_KEY): constants::SESSION_META_TYPE,
            (constants::PAYLOAD_KEY): {
                (constants::SESSION_ID_KEY): test_support::SESSION_ID,
                (constants::CWD_KEY): fixture.cwd,
            }
        }),
    ];
    for conflict in conflicts {
        let records = [accepted.clone(), conflict];
        assert!(RolloutMetadata::from_head(&path, &records, &fixture.cwd).is_none());
    }
}

#[test]
fn metadata_accepts_a_changed_absolute_cwd_and_rejects_unsafe_cwds() {
    let fixture = test_support::Fixture::new();
    let path = fixture.evidence(test_support::SESSION_ID);
    let changed = fixture.cwd.with_file_name(OTHER_CWD_DIRECTORY);
    let value = fixture.metadata(test_support::SESSION_ID, json!(constants::CLI_SOURCE));
    std::fs::create_dir(&changed).unwrap();
    let found = RolloutMetadata::from_head(&path, &[value], &changed).unwrap();
    assert_eq!(found.cwd, fixture.cwd.canonicalize().unwrap());

    for cwd in [
        Path::new(RELATIVE_CWD).to_path_buf(),
        Path::new(PARENT_CWD).to_path_buf(),
    ] {
        let value = json!({
            (constants::RECORD_TYPE_KEY): constants::SESSION_META_TYPE,
            (constants::PAYLOAD_KEY): {
                (constants::SESSION_ID_KEY): test_support::SESSION_ID,
                (constants::CWD_KEY): cwd,
                (constants::SOURCE_KEY): constants::CLI_SOURCE,
            }
        });
        assert!(RolloutMetadata::from_head(&path, &[value], &fixture.cwd).is_none());
    }
    let value = fixture.metadata(test_support::SESSION_ID, json!(constants::CLI_SOURCE));
    assert!(RolloutMetadata::from_head(&path, &[value], Path::new(RELATIVE_CWD)).is_none());
}

#[test]
fn latest_session_index_thread_name_is_scoped_to_its_session() {
    let fixture = test_support::Fixture::new();
    let index = fixture.data_home.join(constants::SESSION_INDEX_FILE);
    let lines = [
        json!({
            (constants::SESSION_INDEX_ID_KEY): test_support::SESSION_ID,
            (constants::SESSION_INDEX_TITLE_KEY): "first title",
        }),
        json!({
            (constants::SESSION_INDEX_ID_KEY): test_support::OTHER_SESSION_ID,
            (constants::SESSION_INDEX_TITLE_KEY): "other session",
        }),
        json!({
            (constants::SESSION_INDEX_ID_KEY): test_support::SESSION_ID,
            (constants::SESSION_INDEX_TITLE_KEY): "最新标题",
        }),
    ];
    let contents = lines
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(index, format!("{contents}\n")).unwrap();
    assert_eq!(
        display_title(&fixture.data_home, test_support::SESSION_ID).as_deref(),
        Some("最新标题")
    );
    assert_eq!(display_title(&fixture.data_home, "missing"), None);
}
