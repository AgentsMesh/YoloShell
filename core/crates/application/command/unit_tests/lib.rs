use super::{Cli, Command, selects};
use clap::Parser;
use constants::*;
use std::ffi::OsString;

#[path = "constants.rs"]
mod constants;

#[test]
fn root_options_before_and_after_a_command_reach_the_same_tree() {
    let before = Cli::try_parse_from(["yolo", LOCAL_PROFILE, PROFILE, AGENT, LIST]).unwrap();
    assert!(matches!(before.command, Command::Agent { .. }));
    let after = Cli::try_parse_from(["yolo", AGENT, LIST, LOCAL_PROFILE, PROFILE]).unwrap();
    assert!(matches!(after.command, Command::Agent { .. }));
}

#[test]
fn selects_recognizes_auxiliary_commands_after_root_options() {
    let arguments = [PROGRAM, LOCAL_PROFILE, PROFILE, AGENT, LIST].map(OsString::from);
    assert!(selects(&arguments));
    let values = [PROGRAM, LOCAL_PROFILE, AGENT, LIST].map(OsString::from);
    assert!(!selects(&values));
}

#[test]
fn commands_reject_connection_options_they_do_not_use() {
    let error = super::run_from(
        ["yolo", PLUGIN, LIST, SOCKET, SOCKET_PATH, HUB, HUB_ID].map(OsString::from),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains(NO_CONNECTION), "{error}");
}

#[test]
fn local_commands_reject_remote_connection_options() {
    let error = super::run_from(
        [
            "yolo",
            ENDPOINT,
            "local:/tmp/yolo.sock",
            HUB,
            HUB_ID,
            AGENT,
            LIST,
        ]
        .map(OsString::from),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains(LOCAL_ONLY), "{error}");
}
