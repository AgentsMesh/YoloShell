use crate::connection::ConnectionOptions;
use crate::constants::*;
use crate::tree::{Command, UpdateArgs};
use anyhow::{Result, ensure};

pub(crate) fn run(connection: ConnectionOptions, command: Command) -> Result<()> {
    match command {
        Command::Agent { command } => {
            local_only(&connection, "agent")?;
            yolo_agent_cli::run_options(yolo_agent_cli::Options {
                socket: connection.socket,
                hub: connection.hub,
                local_profile: connection.local_profile,
                command,
            })
        }
        Command::Conversation { action } => {
            local_only(&connection, "conversation")?;
            yolo_conversation_cli::run(yolo_conversation_cli::Options {
                socket: connection.socket,
                hub: connection.hub,
                local_profile: connection.local_profile,
                action,
            })
        }
        Command::Workspace { command } => {
            local_only(&connection, "workspace")?;
            yolo_workspace_cli::run_workspace(yolo_workspace_cli::Options {
                socket: connection.socket,
                hub: connection.hub,
                local_profile: connection.local_profile,
                command,
            })
        }
        Command::Worktree { command } => {
            local_only(&connection, "worktree")?;
            yolo_workspace_cli::run_worktree(yolo_workspace_cli::WorktreeOptions {
                socket: connection.socket,
                hub: connection.hub,
                local_profile: connection.local_profile,
                command,
            })
        }
        Command::Protocol { command } => {
            no_connection(&connection, "protocol")?;
            yolo_protocol_cli::run_command(command)
        }
        Command::Plugin { command } => {
            no_connection(&connection, "plugin")?;
            yolo_trusted_plugin_cli::run_plugin(command)
        }
        Command::Update(args) => {
            local_only(&connection, "update")?;
            yolo_upgrade_cli::run(yolo_upgrade_cli::Options {
                socket: connection.socket,
                hub: connection.hub,
                state: args.state,
                service_units: args.service_units,
                json: args.json,
                channel: args.channel,
                check: args.check,
                admit: args.admit,
                restart: args.restart,
                no_restart: args.no_restart,
                install_release: args.install_release,
                local_profile: connection.local_profile,
                repository: args.repository,
            })
        }
    }
}

fn local_only(connection: &ConnectionOptions, command: &str) -> Result<()> {
    connection.validate()?;
    ensure!(
        connection.local_only(),
        "{}",
        LOCAL_CONNECTION_ONLY.replace("{command}", command)
    );
    Ok(())
}

fn no_connection(connection: &ConnectionOptions, command: &str) -> Result<()> {
    ensure!(
        !connection.has_any(),
        "{}",
        NO_CONNECTION.replace("{command}", command)
    );
    Ok(())
}
