use super::claude_scenario::HookAvailability;
use super::claude_wire::{CompactTrigger, SessionEndReason, SessionStartSource};
use super::fake_agent_constants::*;

#[test]
fn lifecycle_values_match_the_claude_hook_protocol() {
    assert_names([
        (SessionStartSource::Startup, CLAUDE_STARTUP_SOURCE),
        (SessionStartSource::Resume, CLAUDE_RESUME_SOURCE),
        (SessionStartSource::Clear, CLAUDE_CLEAR_SOURCE),
        (SessionStartSource::Compact, CLAUDE_COMPACT_SOURCE),
        (SessionStartSource::Fork, CLAUDE_FORK_SOURCE),
    ]);
    assert_names([
        (SessionEndReason::Clear, CLAUDE_CLEAR_SOURCE),
        (SessionEndReason::Resume, CLAUDE_RESUME_SOURCE),
        (SessionEndReason::Logout, CLAUDE_LOGOUT_REASON),
        (SessionEndReason::PromptInputExit, CLAUDE_PROMPT_EXIT_REASON),
        (
            SessionEndReason::BypassPermissionsDisabled,
            CLAUDE_BYPASS_DISABLED_REASON,
        ),
        (SessionEndReason::Other, CLAUDE_OTHER_REASON),
    ]);
    assert_names([
        (CompactTrigger::Manual, CLAUDE_MANUAL_TRIGGER),
        (CompactTrigger::Auto, CLAUDE_AUTO_TRIGGER),
    ]);
}

#[test]
fn hook_availability_includes_the_exact_disable_all_hooks_setting() {
    assert_names([
        (HookAvailability::Enabled, CLAUDE_HOOKS_ENABLED),
        (HookAvailability::Missing, CLAUDE_HOOKS_MISSING),
        (HookAvailability::Error, CLAUDE_HOOKS_ERROR),
        (HookAvailability::DisableAllHooks, CLAUDE_HOOKS_DISABLED),
    ]);
}

fn assert_names<T: serde::Serialize, const N: usize>(cases: [(T, &str); N]) {
    for (value, expected) in cases {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
}
