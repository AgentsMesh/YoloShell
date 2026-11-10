use super::ping_marker;
use super::scenario::{Scenario, Step, configured_scenario, environment_name};
use crate::fake_agent_constants::PACKAGED_E2E_ENABLED;

const INEXACT_ENABLED_VALUE: &str = "enabled";
const PING_COMMAND: &str = "yolo-ping left right nonce\r";
const PING_MARKER: &str = "leftrightnonce";
const QUOTED_PING_COMMAND: &str = "yolo-ping 'left' 'right' 'nonce'";
const QUOTED_PING_MARKER: &str = "'left''right''nonce'";
const WRONG_PING_COMMAND: &str = "other left right nonce";
const EXTRA_FIELD_PING_COMMAND: &str = "yolo-ping left right nonce extra";

#[test]
fn explicit_packaged_mode_selects_the_full_scenario() {
    assert_eq!(
        configured_scenario(Some(PACKAGED_E2E_ENABLED)),
        Scenario::Full
    );
}

#[test]
fn native_launch_selects_the_start_only_scenario() {
    assert_eq!(configured_scenario(None), Scenario::StartOnly);
}

#[test]
fn inexact_packaged_configuration_keeps_native_default() {
    assert_eq!(
        configured_scenario(Some(INEXACT_ENABLED_VALUE)),
        Scenario::StartOnly
    );
}

#[test]
fn scenarios_expose_exact_provider_event_plans() {
    assert_eq!(
        Scenario::Full.steps(),
        [
            Step::SessionStart,
            Step::TranscriptRunning,
            Step::UserPrompt,
            Step::TranscriptWaiting,
            Step::Stop,
            Step::WaitingNotification,
        ]
    );
    assert_eq!(Scenario::StartOnly.steps(), [Step::SessionStart]);
}

#[test]
fn scenario_environment_names_are_centralized() {
    assert_eq!(
        environment_name(),
        crate::fake_agent_constants::PACKAGED_E2E_ENV
    );
}

#[test]
fn ping_marker_joins_exact_unquoted_tui_fields() {
    assert_eq!(ping_marker(PING_COMMAND), Some(PING_MARKER.to_owned()));
}

#[test]
fn ping_marker_treats_shell_quotes_as_literal_tui_bytes() {
    assert_eq!(
        ping_marker(QUOTED_PING_COMMAND),
        Some(QUOTED_PING_MARKER.to_owned())
    );
}

#[test]
fn ping_marker_rejects_other_commands_and_extra_fields() {
    for line in [WRONG_PING_COMMAND, EXTRA_FIELD_PING_COMMAND] {
        assert_eq!(ping_marker(line), None, "{line}");
    }
}
