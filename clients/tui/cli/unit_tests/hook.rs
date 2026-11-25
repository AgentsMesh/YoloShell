use super::constants::*;
use crate::arguments::Arguments;
use crate::command::Command;
use clap::{CommandFactory, Parser};
use yolo_tui_cli_hook::HookCommand;

const HOOK_ENTRY: &str = "hook";
const PROVIDER: &str = "demo";

#[test]
fn hook_commands_are_first_class_and_local() {
    assert!(matches!(
        Arguments::try_parse_from([PROGRAM, HOOK_ENTRY, STATUS_COMMAND, JSON_ARGUMENT])
            .unwrap()
            .command,
        Some(Command::Hook {
            command: HookCommand::Status { json: true }
        })
    ));
    let Some(Command::Hook {
        command: HookCommand::Install {
            providers, yes, ..
        },
    }) = Arguments::try_parse_from([
        PROGRAM,
        HOOK_ENTRY,
        INSTALL_COMMAND,
        PROVIDER,
        YES_ARGUMENT,
    ])
    .unwrap()
    .command
    else {
        panic!();
    };
    assert_eq!(providers, [PROVIDER]);
    assert!(yes);
    assert!(Arguments::command().render_long_help().to_string().contains(HOOK_ENTRY));
}
