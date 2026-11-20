mod connection;
mod constants;
mod dispatch;
mod tree;

pub use connection::ConnectionOptions;
pub use constants::{CONNECTION_REQUIRES_HUB, HUB_REQUIRES_CONNECTION};
pub use tree::{Command, UpdateArgs};
pub use yolo_agent_cli::AgentCommand;
pub use yolo_conversation_cli::Action as ConversationAction;
pub use yolo_protocol_cli::ProtocolCommand;
pub use yolo_trusted_plugin_cli::PluginCommand;
pub use yolo_workspace_cli::{WorktreeCommand, WorkspaceCommand};

pub fn run(connection: ConnectionOptions, command: Command) -> anyhow::Result<()> {
    dispatch::run(connection, command)
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
