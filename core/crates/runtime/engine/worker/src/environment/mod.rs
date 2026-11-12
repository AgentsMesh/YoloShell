//! Environment policy for the PTY execution machine, shared by creation and resume.
//! Client environments are not forwarded across Hub boundaries by this policy.

mod constants;

use constants::*;
use portable_pty::CommandBuilder;
use std::collections::BTreeMap;

pub(super) fn configure(command: &mut CommandBuilder, overrides: &BTreeMap<String, String>) {
    // CommandBuilder already snapshots inherited OS strings. Resolve defaults against
    // that same snapshot after overrides, without reading or mutating global state.
    for (key, value) in overrides {
        command.env(key, value);
    }
    for (key, fallback) in CAPABILITY_DEFAULTS {
        if !has_value(command, key) {
            command.env(key, fallback);
        }
    }
    // POSIX locale precedence remains LC_ALL > LC_CTYPE > LANG. A nonempty value
    // at any level is the user's choice, including C/POSIX and non-UTF-8 locales.
    // Only default the character category; other LC_* categories remain untouched.
    if !CHARACTER_LOCALE_KEYS
        .iter()
        .any(|key| has_value(command, key))
    {
        command.env(LC_CTYPE_ENV, DEFAULT_CHARACTER_LOCALE);
    }
}

fn has_value(command: &CommandBuilder, key: &str) -> bool {
    command.get_env(key).is_some_and(|value| !value.is_empty())
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-environment"))]
#[path = "../../unit_tests/environment/policy.rs"]
mod tests;
