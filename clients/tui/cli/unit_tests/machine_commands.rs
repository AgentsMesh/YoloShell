use super::constants::*;
use super::machine::{MACHINE_HUB, MACHINE_KEY};
use crate::arguments::Arguments;
use crate::command::Command;
use clap::Parser;
use yolo_tui_hub_cli::MachineCommand;

fn machine_command(arguments: Vec<&str>) -> MachineCommand {
    let parsed = Arguments::try_parse_from(arguments).unwrap();
    match parsed.command {
        Some(Command::Machine { command }) => command,
        _ => panic!("expected machine command"),
    }
}

#[test]
fn machine_commands_parse_at_the_top_level() {
    assert!(matches!(
        machine_command(vec![PROGRAM, MACHINE_COMMAND, ADD_COMMAND, SSH_TARGET]),
        MachineCommand::Add(_)
    ));
    assert!(matches!(
        machine_command(vec![
            PROGRAM,
            MACHINE_COMMAND,
            PIN_COMMAND,
            MACHINE_HUB,
            MACHINE_KEY
        ]),
        MachineCommand::Pin(_)
    ));
    assert!(matches!(
        machine_command(vec![
            PROGRAM,
            MACHINE_COMMAND,
            IDENTITY_COMMAND,
            JSON_ARGUMENT
        ]),
        MachineCommand::Identity { json: true }
    ));
    assert!(matches!(
        machine_command(vec![PROGRAM, MACHINE_COMMAND, LIST_COMMAND, JSON_ARGUMENT]),
        MachineCommand::List { json: true }
    ));
    assert!(matches!(
        machine_command(vec![
            PROGRAM,
            MACHINE_COMMAND,
            CONSUMERS_COMMAND,
            JSON_ARGUMENT
        ]),
        MachineCommand::Consumers { json: true }
    ));
    assert!(matches!(
        machine_command(vec![
            PROGRAM,
            MACHINE_COMMAND,
            REMOVE_COMMAND,
            MACHINE_HUB,
            JSON_ARGUMENT
        ]),
        MachineCommand::Remove { .. }
    ));
}
