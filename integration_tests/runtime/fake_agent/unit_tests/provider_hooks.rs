use super::*;
use crate::fake_agent_constants::{PACKAGED_HOOK_SESSION_FIELD, PACKAGED_SESSION_START_EVENT};
use crate::packaged_claude_test_support::*;
use serde_json::json;
const COMMAND_TEXT: &str = "exec /usr/bin/true";
const WRONG_KIND: &str = "unknown-kind";

#[test]
fn malformed_configuration_is_not_executed() {
    for root in [
        json!([]),
        json!({HOOKS: []}),
        json!({HOOKS: {"SessionStart": {}}}),
        json!({HOOKS: {"SessionStart": [{}]}}),
        json!({HOOKS: {"SessionStart": [{HOOKS: [{}]}]}}),
    ] {
        assert!(parse(&root).is_err());
    }
    for value in [Value::Null, json!(String::new()), json!(u64::MAX)] {
        let mut settings = valid_settings(COMMAND_TEXT);
        settings[HOOKS][PACKAGED_SESSION_START_EVENT][0][HOOKS][0][COMMAND] = value;
        assert!(parse(&settings).is_err());
    }
}

#[test]
fn old_command_args_contract_and_unknown_hook_kind_are_rejected() {
    for (key, value) in [(ARGUMENTS, json!([])), (TYPE, json!(WRONG_KIND))] {
        let mut settings = valid_settings(COMMAND_TEXT);
        settings[HOOKS][PACKAGED_SESSION_START_EVENT][0][HOOKS][0][key] = value;
        assert!(parse(&settings).is_err());
    }
}

#[test]
fn oversized_and_invalid_json_are_rejected() {
    let home = tempfile::tempdir().unwrap();
    let path = settings_path(home.path());
    std::fs::write(&path, b"{").unwrap();
    assert!(load(&path).is_err());
    std::fs::write(&path, vec![b' '; (MAX_BYTES + READ_SENTINEL) as usize]).unwrap();
    assert!(load(&path).is_err());
}

#[test]
fn shell_execution_retains_space_and_quote_path_boundaries() {
    let home = tempfile::tempdir().unwrap();
    let helper = executable(home.path(), "helper with ' quote", SUCCESS_SCRIPT);
    let capture = home.path().join("capture with ' quote");
    let input = json!({PACKAGED_HOOK_SESSION_FIELD: SESSION_ID});
    execute(&command(&helper, Some(&capture)), &input).unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(capture).unwrap()).unwrap(),
        input
    );
}
