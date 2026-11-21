use super::*;
use std::ffi::OsStr;

#[path = "../../test_support/unit/bootstrap/bootstrap_environment_constants.rs"]
mod constants;
use self::constants::*;

#[test]
fn bootstrap_uses_launch_terminal_and_character_locale_overrides() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let mut launch = launch_spec(&paths, Uuid::new_v4());
    launch.environment = ENVIRONMENT_OVERRIDES
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .into();
    let spec = bootstrap_child_spec(&launch, Uuid::new_v4()).unwrap();
    let spawn = bootstrap_command(&launch, &spec, None, None, None).unwrap();

    for (key, value) in ENVIRONMENT_OVERRIDES {
        assert_eq!(spawn.command.get_env(key), Some(OsStr::new(value)), "{key}");
    }
}

#[test]
fn bootstrap_fills_empty_effective_launch_environment() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let mut launch = launch_spec(&paths, Uuid::new_v4());
    launch.environment = ENVIRONMENT_KEYS
        .map(|key| (key.to_owned(), String::new()))
        .into();
    let spec = bootstrap_child_spec(&launch, Uuid::new_v4()).unwrap();
    let spawn = bootstrap_command(&launch, &spec, None, None, None).unwrap();

    for (key, value) in CAPABILITY_DEFAULTS {
        assert_eq!(spawn.command.get_env(key), Some(OsStr::new(value)), "{key}");
    }
    assert_eq!(
        spawn.command.get_env(LC_CTYPE_ENV),
        Some(OsStr::new(DEFAULT_CTYPE))
    );
}
