use super::constants::*;
use crate::arguments::Arguments;
use crate::command::Command;
use clap::Parser;
use std::ffi::OsString;

#[test]
fn auxiliary_commands_share_the_single_root_tree() {
    let arguments = Arguments::try_parse_from([
        PROGRAM,
        AGENT_COMMAND,
        LIST_COMMAND,
        LOCAL_PROFILE_ARGUMENT,
        PROFILE,
    ])
    .unwrap();
    assert!(matches!(arguments.command, Some(Command::Agent { .. })));
    let error = crate::run_from(
        [
            PROGRAM,
            "plugin",
            LIST_COMMAND,
            SOCKET_ARGUMENT,
            SOCKET,
            HUB_ARGUMENT,
            HUB,
        ]
        .map(OsString::from),
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("does not use Hub connection options"),
        "{error}"
    );
}
