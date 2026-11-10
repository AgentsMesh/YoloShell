use super::packaged_codex_tui::{launch_marker_for_test, ping_marker_for_test};
use crate::fake_agent_constants::{CODEX_TUI_MARKER_ERROR, PACKAGED_TUI_PING_COMMAND};

const LEFT: &str = "left";
const RIGHT: &str = "right";
const NONCE: &str = "nonce";

#[test]
fn stdin_eof_exits_unless_explicit_fixture_lifetime_is_requested() {
    use super::packaged_codex_tui::hold_after_eof;
    use crate::fake_agent_constants::HOLD_AFTER_EOF_ENABLED;
    use std::ffi::OsStr;
    assert!(!hold_after_eof(None));
    assert!(!hold_after_eof(Some(OsStr::new(""))));
    assert!(!hold_after_eof(Some(OsStr::new("0"))));
    assert!(hold_after_eof(Some(OsStr::new(HOLD_AFTER_EOF_ENABLED))));
}

#[test]
fn launch_marker_requires_all_or_none_of_its_components() {
    assert_eq!(launch_marker_for_test([None, None, None]).unwrap(), None);
    assert_eq!(
        launch_marker_for_test([
            Some(LEFT.to_owned()),
            Some(RIGHT.to_owned()),
            Some(NONCE.to_owned()),
        ])
        .unwrap(),
        Some(format!("{LEFT}{RIGHT}{NONCE}"))
    );
    assert_eq!(
        launch_marker_for_test([Some(LEFT.to_owned()), None, Some(NONCE.to_owned())]),
        Err(CODEX_TUI_MARKER_ERROR)
    );
}

#[test]
fn tui_ping_accepts_only_the_exact_command_shape() {
    let command = format!("{PACKAGED_TUI_PING_COMMAND} {LEFT} {RIGHT} {NONCE}");
    assert_eq!(
        ping_marker_for_test(&command),
        Some(format!("{LEFT}{RIGHT}{NONCE}"))
    );
    assert_eq!(ping_marker_for_test(PACKAGED_TUI_PING_COMMAND), None);
    assert_eq!(ping_marker_for_test(&format!("{command} extra")), None);
}
