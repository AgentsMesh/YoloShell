use super::constants::*;
use crate::arguments::Arguments;
use crate::command::Command;
use clap::Parser;
use std::ffi::OsString;

const PLUGIN: &str = "plugin";
const COMMAND_HELP: &str = "help";
const DATA_DIR: &str = "data-dir";
const OTHER_COMMANDS: &str = "More commands";

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
            PLUGIN,
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

#[test]
fn bare_plugin_selects_the_same_catalogue_as_list() {
    for args in [vec![PROGRAM, PLUGIN], vec![PROGRAM, PLUGIN, LIST_COMMAND]] {
        let parsed = Arguments::try_parse_from(args).unwrap();
        let Some(Command::Plugin(options)) = parsed.command else {
            panic!()
        };
        assert!(matches!(options.into_command(), yolo_command_cli::PluginCommand::List {
            json: false, installed: false, registry: None,
        }));
    }
}

#[test]
fn plugin_help_prioritizes_user_commands_and_scopes_their_options() {
    let help = |args: &[&str]| {
        let error = Arguments::try_parse_from(args).unwrap_err();
        assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
        error.to_string()
    };
    let group = help(&[PROGRAM, PLUGIN, HELP_ARGUMENT]);
    assert_eq!(help(&[PROGRAM, COMMAND_HELP, PLUGIN]), group);
    let (common, other) = group.split_once(OTHER_COMMANDS).unwrap();
    for name in [LIST_COMMAND, INSTALL_COMMAND, "startup", "log"] {
        assert!(common.lines().any(|line| line.split_whitespace().next() == Some(name)));
    }
    assert!(other.contains(DATA_DIR));
    let advanced = help(&[PROGRAM, PLUGIN, COMMAND_HELP, DATA_DIR]);
    let install = help(&[PROGRAM, PLUGIN, INSTALL_COMMAND, HELP_ARGUMENT]);
    assert_eq!(help(&[PROGRAM, COMMAND_HELP, PLUGIN, INSTALL_COMMAND]), install);
    for text in [&group, &advanced, &install] {
        assert!(!text.contains(SOCKET_ARGUMENT), "{text}");
        assert!(!text.contains(LAYOUT_ARGUMENT), "{text}");
    }
    assert!(advanced.contains("<ID>"));
    assert!(install.contains(JSON_ARGUMENT));
    assert!(help(&[PROGRAM, AGENT_COMMAND, LIST_COMMAND, HELP_ARGUMENT]).contains(SOCKET_ARGUMENT));
}
