#[path = "../test_support/bootstrap_dispatch.rs"]
mod fixture;
use std::ffi::OsString;
use yolo_launch_contract::{EXECUTABLE_NAME, INTERNAL_COMMAND, ProcessRole};

#[path = "bootstrap_dispatch_constants.rs"]
mod constants;
use constants::*;

#[test]
fn default_cli_arguments_reach_the_real_tui_entry_without_selecting_hub() {
    let expected = [EXECUTABLE_NAME, PROFILE_ARGUMENT, PROFILE_PATH].map(OsString::from);
    fixture::dispatch(
        [ALIASED_NAME, PROFILE_ARGUMENT, PROFILE_PATH].map(OsString::from),
        |arguments| {
            assert_eq!(arguments, expected);
            Ok(())
        },
        |_| panic!("default invocation selected the fake Hub"),
    )
    .unwrap();
}

#[test]
fn only_the_exact_explicit_hub_role_reaches_the_fake_bootstrap() {
    let mut arguments = vec![OsString::from(ALIASED_NAME)];
    arguments.extend(ProcessRole::Hub.arguments().map(OsString::from));
    arguments.push(PROFILE_PATH.into());
    fixture::dispatch(
        arguments,
        |_| panic!("explicit Hub invocation selected the TUI"),
        |arguments| {
            assert_eq!(
                arguments,
                [ProcessRole::Hub.display_name(), PROFILE_PATH].map(OsString::from)
            );
            Ok(())
        },
    )
    .unwrap();
    for role in ProcessRole::ALL
        .iter()
        .filter(|role| **role != ProcessRole::Hub)
    {
        assert!(
            fixture::dispatch(
                [EXECUTABLE_NAME, INTERNAL_COMMAND, role.name()].map(OsString::from),
                |_| panic!(),
                |_| panic!(),
            )
            .is_err()
        );
    }
    assert!(
        fixture::dispatch(
            [EXECUTABLE_NAME, INTERNAL_COMMAND, UNKNOWN_ROLE].map(OsString::from),
            |_| panic!(),
            |_| panic!(),
        )
        .is_err()
    );
}
