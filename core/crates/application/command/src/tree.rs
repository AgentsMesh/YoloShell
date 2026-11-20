use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Inspect and control a specific agent through the public Hub protocol.
    Agent {
        #[command(subcommand)]
        command: yolo_agent_cli::AgentCommand,
    },
    /// Read native agent conversations through the public Hub protocol.
    Conversation {
        #[command(subcommand)]
        action: yolo_conversation_cli::Action,
    },
    /// Manage durable workspace metadata through the public Hub.
    Workspace {
        #[command(subcommand)]
        command: yolo_workspace_cli::WorkspaceCommand,
    },
    /// Manage git worktrees through the public Hub.
    Worktree {
        #[command(subcommand)]
        command: yolo_workspace_cli::WorktreeCommand,
    },
    /// Inspect the public Yolo protocol contract.
    Protocol {
        #[command(subcommand)]
        command: yolo_protocol_cli::ProtocolCommand,
    },
    /// Manage trusted user-level plugins.
    Plugin {
        #[command(subcommand)]
        command: yolo_trusted_plugin_cli::PluginCommand,
    },
    /// Check, install and activate Yolo releases.
    Update(UpdateArgs),
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Print one JSON document.
    #[arg(long)]
    pub json: bool,
    /// Release channel to select.
    #[arg(long, value_enum, default_value = "stable")]
    pub channel: yolo_upgrade_cli::Channel,
    /// Check the latest release without installing or restarting.
    #[arg(long, alias = "check-release", conflicts_with_all = ["install_release", "restart", "admit", "no_restart", "validate_state"])]
    pub check: bool,
    /// Admit the already installed image without checking releases.
    #[arg(long, conflicts_with_all = ["install_release", "restart", "check", "no_restart", "validate_state"])]
    pub admit: bool,
    /// Restart the local Host and Hub, then admit the installed image.
    #[arg(long, conflicts_with_all = ["install_release", "check", "admit", "no_restart", "validate_state"])]
    pub restart: bool,
    /// Install a newer release without restarting; only with the default update.
    #[arg(long, conflicts_with_all = ["install_release", "check", "admit", "restart", "validate_state"])]
    pub no_restart: bool,
    /// Legacy install-only mode.
    #[arg(long, hide = true, conflicts_with_all = ["check", "admit", "restart", "no_restart", "validate_state"])]
    pub install_release: bool,
    /// Validate the local Hub state contract with this image without changing it.
    #[arg(long, hide = true, conflicts_with_all = ["install_release", "check", "admit", "restart", "no_restart"])]
    pub validate_state: bool,
    #[arg(long, hide = true, value_name = "PATH")]
    pub state: Option<PathBuf>,
    #[arg(long = "service-unit", hide = true, value_name = "PATH")]
    pub service_units: Vec<PathBuf>,
    #[arg(long, hide = true)]
    pub repository: Option<String>,
}
