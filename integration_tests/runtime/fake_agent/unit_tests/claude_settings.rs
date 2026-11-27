use crate::packaged_claude_test_support::*;
use crate::{fake_agent_constants::*, packaged_claude_settings};
use serde_json::json;
const COMMAND: &str = "exec /usr/bin/true";
const SECOND_COMMAND: &str = "exec /usr/bin/false";

#[test]
fn normal_settings_json_is_loaded_without_launch_options() {
    let directory = tempfile::tempdir().unwrap();
    write_json(&settings_path(directory.path()), &valid_settings(COMMAND));
    let loaded = packaged_claude_settings::load(directory.path()).unwrap();
    assert_eq!(loaded.hooks.len(), PACKAGED_HOOK_EVENTS.len());
    for commands in loaded.hooks.values() {
        assert_eq!(commands[0].command, COMMAND);
    }
}

#[test]
fn real_installer_configuration_is_accepted_without_argument_injection() {
    let directory = tempfile::tempdir().unwrap();
    let installed =
        yolo_hook_installation::merge(&yolo_hook_installation_catalog::CLAUDE, &json!({}))
            .unwrap();
    write_json(&settings_path(directory.path()), &installed);
    let loaded = packaged_claude_settings::load(directory.path()).unwrap();
    assert_eq!(loaded.hooks.len(), PACKAGED_HOOK_EVENTS.len());
    for commands in loaded.hooks.values() {
        assert_eq!(
            commands[0].command,
            yolo_hook_installation::callback_command(&yolo_hook_installation_catalog::CLAUDE)
        );
    }
}

#[test]
fn user_hook_groups_and_unknown_events_are_retained() {
    let directory = tempfile::tempdir().unwrap();
    let mut settings = valid_settings(COMMAND);
    settings[PACKAGED_SETTINGS_HOOKS_FIELD][EXTRA_HOOK_EVENT] = hook_event(SECOND_COMMAND);
    let group = hook_event(SECOND_COMMAND).as_array().unwrap()[0].clone();
    settings[PACKAGED_SETTINGS_HOOKS_FIELD][PACKAGED_SESSION_START_EVENT]
        .as_array_mut()
        .unwrap()
        .push(group);
    write_json(&settings_path(directory.path()), &settings);
    let loaded = packaged_claude_settings::load(directory.path()).unwrap();
    let commands = &loaded.hooks[PACKAGED_SESSION_START_EVENT];
    assert_eq!(
        commands
            .iter()
            .map(|command| command.command.as_str())
            .collect::<Vec<_>>(),
        [COMMAND, SECOND_COMMAND]
    );
    assert!(loaded.hooks.contains_key(EXTRA_HOOK_EVENT));
}

#[test]
fn missing_configuration_is_a_normal_unintegrated_provider() {
    let directory = tempfile::tempdir().unwrap();
    assert!(
        packaged_claude_settings::load(directory.path())
            .unwrap()
            .hooks
            .is_empty()
    );
}
