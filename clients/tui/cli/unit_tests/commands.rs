use super::constants::*;
use crate::constants::TERMINALS_COMMAND;
use crate::{
    arguments::Arguments,
    command::{self, Command},
};
use clap::{CommandFactory, Parser};
use yolo_tui_app::StartupAction;

#[test]
fn public_help_explains_tabs_and_detaching_without_workspaces() {
    let help = Arguments::command().render_long_help().to_string();
    for command in [
        ATTACH_COMMAND,
        DETACH_COMMAND,
        NEW_COMMAND,
        LIST_COMMAND,
        CLIENTS_COMMAND,
        TERMINALS_COMMAND,
        DETACH_SHORTCUT,
    ] {
        assert!(help.contains(command), "missing {command}");
    }
    assert!(!help.contains("workspace"));
    assert!(!Arguments::command().get_subcommands().any(|command| {
        command.get_name() == RETIRED_TABS_COMMAND
            || command.get_name() == RETIRED_PICKER_COMMAND
            || command.get_name() == RETIRED_PANELS_COMMAND
    }));
    let attach = Arguments::try_parse_from([PROGRAM, ATTACH_COMMAND, HELP_ARGUMENT]).unwrap_err();
    assert_eq!(attach.kind(), clap::error::ErrorKind::DisplayHelp);
    assert!(attach.to_string().contains(TERMINAL_SELECTOR_LABEL));
}

#[test]
fn terminals_are_explicit_and_retired_navigation_commands_have_no_aliases() {
    let arguments = Arguments::try_parse_from([PROGRAM, TERMINALS_COMMAND]).unwrap();
    assert!(matches!(arguments.command, Some(Command::Terminals)));
    assert_eq!(
        command::startup(arguments.command, false).unwrap(),
        StartupAction::TerminalPicker
    );
    assert!(Arguments::try_parse_from([PROGRAM, RETIRED_PICKER_COMMAND]).is_err());
    assert!(Arguments::try_parse_from([PROGRAM, RETIRED_TABS_COMMAND]).is_err());
    assert!(Arguments::try_parse_from([PROGRAM, RETIRED_PANELS_COMMAND]).is_err());
}

#[test]
fn commands_accept_connection_options_before_or_after_the_command() {
    for arguments in [
        vec![
            PROGRAM,
            LOCAL_PROFILE_ARGUMENT,
            PROFILE,
            ATTACH_COMMAND,
            "shell",
        ],
        vec![
            PROGRAM,
            ATTACH_COMMAND,
            "shell",
            LOCAL_PROFILE_ARGUMENT,
            PROFILE,
        ],
    ] {
        let args = Arguments::try_parse_from(arguments).unwrap();
        assert!(
            matches!(args.command, Some(Command::Attach { terminal: Some(ref name) }) if name == "shell")
        );
        assert!(args.selection().is_ok());
    }
    for name in [
        ATTACH_COMMAND,
        DETACH_COMMAND,
        LIST_COMMAND,
        CLIENTS_COMMAND,
        NEW_COMMAND,
        TERMINALS_COMMAND,
    ] {
        assert!(
            Arguments::try_parse_from([PROGRAM, name, HUB_ARGUMENT, HUB, SOCKET_ARGUMENT, SOCKET])
                .is_ok()
        );
    }
}

#[test]
fn local_and_remote_shell_defaults_are_in_the_correct_machine_scope() {
    let StartupAction::Automatic { cwd } = command::startup(None, false).unwrap() else {
        panic!()
    };
    assert_eq!(cwd, std::env::current_dir().unwrap().to_str().unwrap());
    assert_eq!(
        command::startup(None, true).unwrap(),
        StartupAction::Automatic { cwd: "~".into() }
    );
    assert!(
        command::startup(
            Some(Command::New {
                cwd: Some("relative".into()),
                name: None
            }),
            false
        )
        .is_err()
    );
    assert!(
        command::startup(
            Some(Command::New {
                cwd: Some("/bad\npath".into()),
                name: None
            }),
            true
        )
        .is_err()
    );
}

#[test]
fn attach_is_not_automatic_and_new_preserves_explicit_directory_and_name() {
    assert_eq!(
        command::startup(Some(Command::Attach { terminal: None }), false).unwrap(),
        StartupAction::Attach { selector: None }
    );
    let args = Arguments::try_parse_from([
        PROGRAM,
        NEW_COMMAND,
        CWD_ARGUMENT,
        "/tmp",
        NAME_ARGUMENT,
        "agent",
    ])
    .unwrap();
    assert_eq!(
        command::startup(args.command, true).unwrap(),
        StartupAction::NewShell {
            cwd: "/tmp".into(),
            name: Some("agent".into())
        }
    );
}

#[test]
fn external_detach_requires_an_unambiguous_client_identity() {
    use yolo_tui_tab::ClientDirectory;
    let clients = vec![
        ClientDirectory {
            id: uuid::Uuid::new_v4(),
            path: "/first".into(),
        },
        ClientDirectory {
            id: uuid::Uuid::new_v4(),
            path: "/second".into(),
        },
    ];
    assert!(crate::detach::choose(&[], None).is_err());
    assert!(crate::detach::choose(&clients, None).is_err());
    assert!(crate::detach::choose(&clients, Some(uuid::Uuid::new_v4())).is_err());
    assert_eq!(
        crate::detach::choose(&clients, Some(clients[1].id))
            .unwrap()
            .id,
        clients[1].id
    );
    assert_eq!(
        crate::detach::choose(&clients[..1], None).unwrap().id,
        clients[0].id
    );
}
