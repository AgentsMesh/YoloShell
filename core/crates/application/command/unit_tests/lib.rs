use super::{Command, ConnectionOptions};
use constants::*;
use yolo_agent_cli::{AgentCommand, ListArgs, OutputArgs};
use yolo_hub_client::HubEndpoint;
use yolo_hub_identity::HubId;

#[path = "constants.rs"]
mod constants;

fn connection() -> ConnectionOptions {
    ConnectionOptions {
        socket: Some(SOCKET_PATH.into()),
        hub: Some(HubId::new()),
        ..ConnectionOptions::default()
    }
}

#[test]
fn commands_reject_connection_options_they_do_not_use() {
    let error = super::run(
        connection(),
        Command::Plugin {
            command: yolo_trusted_plugin_cli::PluginCommand::List { json: true },
        },
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains(NO_CONNECTION), "{error}");
}

#[test]
fn local_commands_reject_remote_connection_options() {
    let connection = ConnectionOptions {
        endpoint: Some(HubEndpoint::local(SOCKET_PATH)),
        hub: Some(HubId::new()),
        ..ConnectionOptions::default()
    };
    let command = Command::Agent {
        command: AgentCommand::List(ListArgs {
            all: false,
            output: OutputArgs { json: true },
        }),
    };
    let error = super::run(connection, command).unwrap_err().to_string();
    assert!(error.contains(LOCAL_ONLY), "{error}");
}
