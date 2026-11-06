use super::*;
use std::collections::BTreeMap;
use yolo_hook_installation_contract::{HookEventSpec, HookInstallSpec};

const SPEC: HookInstallSpec = HookInstallSpec {
    provider: "demo",
    display_name: "Demo",
    file_name: "settings.json",
    subdirectory: "",
    environment_name: "DEMO_HOME",
    directory_name: ".demo",
    hook_arguments: "provider-hook --provider demo",
    managed_hook_commands: &["provider-hook --provider demo"],
    events: &[HookEventSpec::new("SessionStart")],
    restricted_keys: &[],
    disabled_paths: &[],
    requires_provider_trust: false,
};

fn lookup<'a>(
    values: &'a BTreeMap<&'static str, &'static str>,
) -> impl Fn(&str) -> Option<OsString> + 'a {
    move |name| values.get(name).map(|value| OsString::from(*value))
}

#[test]
fn explicit_config_home_wins_over_environment() {
    let values = BTreeMap::from([("DEMO_HOME", "/env/demo"), ("HOME", "/user")]);
    let home = resolve_home(&SPEC, Some("/tmp/explicit".into()), lookup(&values)).unwrap();
    assert_eq!(home, PathBuf::from("/tmp/explicit"));
}

#[test]
fn provider_environment_wins_over_the_user_home() {
    let values = BTreeMap::from([("DEMO_HOME", "/env/demo"), ("HOME", "/user")]);
    let home = resolve_home(&SPEC, None, lookup(&values)).unwrap();
    assert_eq!(home, PathBuf::from("/env/demo"));
}

#[test]
fn empty_environment_falls_back_to_the_user_home() {
    let values = BTreeMap::from([("DEMO_HOME", ""), ("HOME", "/user")]);
    let home = resolve_home(&SPEC, None, lookup(&values)).unwrap();
    assert_eq!(home, PathBuf::from("/user/.demo"));
}

#[test]
fn subdirectory_is_appended_after_resolution() {
    let nested = HookInstallSpec {
        subdirectory: "hooks",
        ..SPEC
    };
    let values = BTreeMap::from([("HOME", "/user")]);
    let home = resolve_home(&nested, None, lookup(&values)).unwrap();
    assert_eq!(home, PathBuf::from("/user/.demo/hooks"));
}

#[test]
fn missing_every_home_is_an_error() {
    let values = BTreeMap::new();
    assert!(resolve_home(&SPEC, None, lookup(&values)).is_err());
}
