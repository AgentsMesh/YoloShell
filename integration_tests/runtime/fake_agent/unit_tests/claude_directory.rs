use super::*;
use std::path::Path;

const CURRENT: &str = "/workspace/one";
const CHANGED: &str = "/workspace/two with spaces ";
const RELATIVE: &str = "relative/project";
const PARENT: &str = "/workspace/../other";
const SESSION: &str = "550e8400-e29b-41d4-a716-446655440000";
const TRANSCRIPT: &str = "/workspace/claude/session.jsonl";

#[test]
fn directory_command_preserves_json_path_text_and_rejects_relative_or_parent_paths() {
    let command = format!(
        "{PACKAGED_TUI_CWD_COMMAND} {}",
        serde_json::to_string(CHANGED).unwrap()
    );
    assert_eq!(directory(&command).unwrap(), Some(PathBuf::from(CHANGED)));
    for invalid in [RELATIVE, PARENT] {
        let command = format!(
            "{PACKAGED_TUI_CWD_COMMAND} {}",
            serde_json::to_string(invalid).unwrap()
        );
        assert!(directory(&command).is_err());
    }
    assert!(directory(PACKAGED_TUI_PING_COMMAND).unwrap().is_none());
    assert!(
        directory(&format!("{PACKAGED_TUI_PING_COMMAND} {CURRENT}"))
            .unwrap()
            .is_none()
    );
    assert!(directory(&format!("{PACKAGED_TUI_CWD_COMMAND} {CURRENT}")).is_err());
    let oversized = format!(
        "{PACKAGED_TUI_CWD_COMMAND} {}",
        CURRENT.repeat(MAX_DIRECTORY_COMMAND_BYTES)
    );
    assert!(directory(&oversized).is_err());
}

#[test]
fn cwd_changed_hook_uses_distinct_old_and_actual_new_directories() {
    let input = claude_provider_input::cwd_changed(
        SESSION,
        Path::new(TRANSCRIPT),
        Path::new(CURRENT),
        Path::new(CHANGED),
    )
    .unwrap();
    assert_eq!(
        input[PACKAGED_HOOK_EVENT_FIELD],
        yolo_runtime::claude_hooks::CLAUDE_CWD_CHANGED_EVENT
    );
    assert_eq!(input[PACKAGED_HOOK_SESSION_FIELD], SESSION);
    assert_eq!(input[PACKAGED_HOOK_TRANSCRIPT_FIELD], TRANSCRIPT);
    assert_eq!(input[PACKAGED_HOOK_CWD_FIELD], CHANGED);
    assert_eq!(input[PACKAGED_HOOK_OLD_CWD_FIELD], CURRENT);
    assert_eq!(input[PACKAGED_HOOK_NEW_CWD_FIELD], CHANGED);
}
