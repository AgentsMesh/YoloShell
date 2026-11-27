use crate::{commands, fixtures::*};
use std::time::Duration;
use yolo_hub_federation_settings::Configuration;
use yolo_hub_management_protocol::{ConfigurationChange, FIRST_CONFIGURATION_REVISION};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::TerminalAction;
use yolo_session_timing::*;

fn lifecycle_commands() -> Vec<(Command, Duration)> {
    let mut commands = vec![
        (
            Command::Create { request: create() },
            SESSION_STARTUP_CLIENT_TIMEOUT,
        ),
        (
            Command::TerminalRun {
                request: terminal_run(),
            },
            SESSION_STARTUP_CLIENT_TIMEOUT,
        ),
        (
            Command::AgentStart {
                request: agent_start(),
            },
            SESSION_STARTUP_CLIENT_TIMEOUT,
        ),
        (
            Command::Finalize {
                creation_token: id(constants::CREATION_ID),
                creation: creation(),
            },
            SESSION_LIFECYCLE_CLIENT_TIMEOUT,
        ),
        (
            Command::CleanupTerminate {
                creation: creation(),
                force: false,
            },
            SESSION_LIFECYCLE_CLIENT_TIMEOUT,
        ),
        (
            Command::CleanupArchive {
                creation: creation(),
            },
            SESSION_LIFECYCLE_CLIENT_TIMEOUT,
        ),
    ];
    for (action, downstream) in [
        (TerminalAction::Resume {}, SESSION_RESUME_CLIENT_TIMEOUT),
        (TerminalAction::Recreate {}, SESSION_RESUME_CLIENT_TIMEOUT),
        (
            TerminalAction::Terminate { force: false },
            SESSION_LIFECYCLE_CLIENT_TIMEOUT,
        ),
        (
            TerminalAction::Terminate { force: true },
            SESSION_LIFECYCLE_CLIENT_TIMEOUT,
        ),
        (TerminalAction::Archive {}, SESSION_LIFECYCLE_CLIENT_TIMEOUT),
    ] {
        let mut command = session_command();
        command.action = action;
        commands.push((Command::Terminal { command }, downstream));
    }
    commands
}

#[test]
fn lifecycle_floors_allow_host_completion_and_never_shorten_a_larger_configured_budget() {
    for (command, downstream) in lifecycle_commands() {
        let expected = downstream + REMOTE_OPERATION_RESPONSE_MARGIN;
        assert_eq!(command.operation_timeout(Duration::ZERO), expected);
        assert_eq!(command.operation_timeout(downstream), expected);
        assert_eq!(command.operation_timeout(expected), expected);
        let larger = expected + RUST_INSPECTION_CLIENT_TIMEOUT;
        assert_eq!(command.operation_timeout(larger), larger);
    }
}

#[test]
fn non_lifecycle_commands_preserve_even_short_callers_budgets() {
    let mut commands: Vec<_> = commands::samples()
        .into_iter()
        .map(|(command, _)| command)
        .collect();
    commands.extend([
        Command::GetBuild {},
        Command::GetConfiguration {},
        Command::GetSshProfiles {},
        Command::InspectRemoteSetup {
            operation_id: id(constants::REQUEST_ID),
        },
        Command::Configure {
            change: ConfigurationChange {
                request_id: id(constants::REQUEST_ID),
                expected_revision: FIRST_CONFIGURATION_REVISION,
                configuration: Configuration::default(),
            },
        },
    ]);
    for command in commands.into_iter().filter(|command| {
        !matches!(
            command,
            Command::Create { .. }
                | Command::TerminalRun { .. }
                | Command::AgentStart { .. }
                | Command::AgentStop { .. }
                | Command::Finalize { .. }
                | Command::Terminal { .. }
                | Command::CleanupTerminate { .. }
                | Command::CleanupArchive { .. }
        )
    }) {
        for configured in [
            Duration::ZERO,
            REMOTE_OPERATION_RESPONSE_MARGIN,
            Duration::MAX,
        ] {
            assert_eq!(
                command.operation_timeout(configured),
                configured,
                "{command:?} configured={configured:?}"
            );
        }
    }
}
