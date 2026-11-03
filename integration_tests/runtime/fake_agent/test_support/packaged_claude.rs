use crate::fake_agent_constants::*;
use serde_json::{Map, Value, json};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub(crate) const SESSION_ID: &str = "123e4567-e89b-42d3-a456-426614174000";
pub(crate) const RUNTIME_EPOCH: &str = "223e4567-e89b-42d3-a456-426614174000";
pub(crate) const EXTRA_HOOK_EVENT: &str = "FutureHook";
pub(crate) const SETTINGS_FILE_NAME: &str = "settings.json";
pub(crate) const HELPER_FILE_NAME: &str = "hook-helper";
pub(crate) const CAPTURE_FILE_NAME: &str = "hook-input.json";
pub(crate) const TRANSCRIPT_FILE_NAME: &str = "transcript.jsonl";
pub(crate) const TEST_EXECUTABLE_MODE: u32 = 0o700;
pub(crate) const PERMISSION_MODE_MASK: u32 = 0o777;
pub(crate) const SUCCESS_SCRIPT: &[u8] = b"#!/bin/sh\n/bin/cat > \"$1\"\n";
pub(crate) const FAILURE_SCRIPT: &[u8] = b"#!/bin/sh\nexit 23\n";

pub(crate) fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

pub(crate) fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

pub(crate) fn settings_path(directory: &Path) -> PathBuf {
    directory.join(SETTINGS_FILE_NAME)
}

pub(crate) fn valid_settings(command: &str) -> Value {
    let mut hooks = Map::new();
    for &(event, _) in PACKAGED_HOOK_EVENTS {
        hooks.insert(event.to_owned(), hook_event(command));
    }
    json!({(PACKAGED_SETTINGS_HOOKS_FIELD): hooks})
}

pub(crate) fn hook_event(command: &str) -> Value {
    json!([{
        (PACKAGED_SETTINGS_MATCHER_FIELD): PACKAGED_SETTINGS_MATCH_ALL,
        (PACKAGED_SETTINGS_HOOKS_FIELD): [{
            (PACKAGED_SETTINGS_TYPE_FIELD): PACKAGED_SETTINGS_COMMAND_TYPE,
            (PACKAGED_SETTINGS_COMMAND_FIELD): command,
        }],
    }])
}

pub(crate) fn command(helper: &Path, capture: Option<&Path>) -> crate::provider_hooks::HookCommand {
    let quoted = |path: &Path| format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"));
    let command = match capture {
        Some(capture) => format!("exec {} {}", quoted(helper), quoted(capture)),
        None => format!("exec {}", quoted(helper)),
    };
    crate::provider_hooks::HookCommand { command }
}

pub(crate) fn executable(directory: &Path, name: &str, contents: &[u8]) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, contents).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(TEST_EXECUTABLE_MODE)).unwrap();
    path
}

pub(crate) fn json_lines(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
