use super::constants::*;
use super::environment::NativeEnvironment;
use super::test_support::{Fixture, request};
use std::collections::BTreeMap;
use std::ffi::OsString;
#[path = "../../test_support/executable_bounds.rs"]
mod bounds;

const SOCKET_NAME: &str = "native.sock";
const INVALID_VALUE: &str = "invalid";

fn values(fixture: &Fixture) -> BTreeMap<&'static str, OsString> {
    BTreeMap::from([
        (
            NATIVE_HOOK_HOME_ENV,
            fixture.paths.home().as_os_str().to_owned(),
        ),
        (
            NATIVE_HOOK_EXECUTABLE_ENV,
            fixture.request.hook_executable.as_os_str().to_owned(),
        ),
        (
            TERMINAL_ENV,
            fixture.identity.terminal_id.to_string().into(),
        ),
        (
            RUNTIME_ENV,
            fixture.identity.runtime_epoch.to_string().into(),
        ),
        (
            SOCKET_ENV,
            fixture.paths.home().join(SOCKET_NAME).into_os_string(),
        ),
        (
            NATIVE_DATA_HOME_ENV,
            fixture.request.data_home.as_os_str().to_owned(),
        ),
    ])
}

#[test]
fn native_environment_uses_only_generic_session_context_not_invocation_secrets() {
    let fixture = Fixture::new();
    let values = values(&fixture);
    let environment =
        NativeEnvironment::from_values(&fixture.paths, &fixture.request.hook_executable, |name| {
            assert!(values.contains_key(name) || name == NATIVE_USER_HOME_ENV);
            values.get(name).cloned()
        })
        .unwrap();
    let first = environment.request(request(&fixture).input).unwrap();
    let second = environment.request(request(&fixture).input).unwrap();
    assert_eq!(first.terminal_id, fixture.identity.terminal_id);
    assert_eq!(first.runtime_epoch, fixture.identity.runtime_epoch);
    assert_ne!(first.event_id, second.event_id);
    assert_eq!(
        first.data_home_candidates,
        vec![fixture.request.data_home.clone()]
    );
}

#[test]
fn native_environment_rejects_missing_context_and_changed_callback_target() {
    let fixture = Fixture::new();
    for key in [
        NATIVE_HOOK_HOME_ENV,
        NATIVE_HOOK_EXECUTABLE_ENV,
        TERMINAL_ENV,
        RUNTIME_ENV,
        SOCKET_ENV,
        NATIVE_DATA_HOME_ENV,
    ] {
        let mut values = values(&fixture);
        values.remove(key);
        assert!(
            NativeEnvironment::from_values(
                &fixture.paths,
                &fixture.request.hook_executable,
                |name| values.get(name).cloned()
            )
            .is_err()
        );
    }
    let values = values(&fixture);
    assert!(
        NativeEnvironment::from_values(&fixture.paths, &fixture.request.real_executable, |name| {
            values.get(name).cloned()
        })
        .is_err()
    );
    for key in [
        NATIVE_HOOK_HOME_ENV,
        NATIVE_HOOK_EXECUTABLE_ENV,
        TERMINAL_ENV,
        RUNTIME_ENV,
        SOCKET_ENV,
        NATIVE_DATA_HOME_ENV,
    ] {
        let mut values = values.clone();
        values.insert(key, INVALID_VALUE.into());
        assert!(
            NativeEnvironment::from_values(
                &fixture.paths,
                &fixture.request.hook_executable,
                |name| values.get(name).cloned()
            )
            .is_err()
        );
    }
}

#[test]
fn native_environment_supports_default_home_and_deduplicates_candidates() {
    let fixture = Fixture::new();
    let mut values = values(&fixture);
    values.remove(NATIVE_DATA_HOME_ENV);
    values.insert(
        NATIVE_USER_HOME_ENV,
        fixture.paths.home().as_os_str().to_owned(),
    );
    let expected = fixture.paths.home().join(NATIVE_DATA_DIRECTORY_NAME);
    let environment =
        NativeEnvironment::from_values(&fixture.paths, &fixture.request.hook_executable, |name| {
            values.get(name).cloned()
        })
        .unwrap();
    assert_eq!(environment.data_home_candidates, vec![expected.clone()]);
    values.insert(NATIVE_DATA_HOME_ENV, expected.clone().into_os_string());
    let environment =
        NativeEnvironment::from_values(&fixture.paths, &fixture.request.hook_executable, |name| {
            values.get(name).cloned()
        })
        .unwrap();
    assert_eq!(environment.data_home_candidates, vec![expected]);
}

#[test]
fn native_callback_uses_the_runtime_bound_before_sending_its_first_request() {
    let fixture = Fixture::new();
    let _file = bounds::resize(
        &fixture.request.hook_executable,
        bounds::RUNTIME_SIZED_EXECUTABLE,
    );
    let values = values(&fixture);
    let environment =
        NativeEnvironment::from_values(&fixture.paths, &fixture.request.hook_executable, |name| {
            values.get(name).cloned()
        })
        .unwrap();
    bounds::assert_invalid(
        environment
            .executable
            .verify_current(crate::codex_hooks::CodexExecutableKind::Provider),
    );
    let request = environment.request(request(&fixture).input).unwrap();
    assert_eq!(request.hook_executable, fixture.request.hook_executable);
}
