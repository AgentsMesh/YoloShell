use crate::client::ClientCommand;
use crate::constants::*;
use crate::layouts::LayoutCommand;
use yolo_tui_cli_platform::PlatformCommand;
use crate::recipe::RecipeCommand;
use crate::server::ServerCommand;
use crate::terminal::TerminalCommand;
use clap::Subcommand;
use yolo_command_cli::{
    AgentCommand, ConversationAction, PluginCommand, ProtocolCommand, UpdateArgs, WorkspaceCommand,
    WorktreeCommand,
};
use yolo_tui_app::StartupAction;

/// The single root command tree. Daily shell entry stays first; every other
/// command is grouped by the noun it operates on.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Open a new tab with a normal shell. Run agents directly in that shell.
    #[command(group(crate::arguments::connection_group()))]
    New {
        /// Working directory for the new shell; defaults to the current directory.
        #[arg(long)]
        cwd: Option<String>,
        /// Label for the terminal; selectors need the label to be unambiguous.
        #[arg(long)]
        name: Option<String>,
    },
    /// Attach an existing terminal by ID, address, or unique name; never creates a shell.
    #[command(group(crate::arguments::connection_group()))]
    Attach {
        /// Terminal id, address, or unique name.
        terminal: Option<String>,
    },
    /// Open the terminal picker to attach an existing shell without creating one.
    #[command(name = TERMINALS_COMMAND, group(crate::arguments::connection_group()))]
    Terminals,
    /// List, read, archive or terminate Hub-owned terminals.
    #[command(group(crate::arguments::connection_group()))]
    Terminal {
        #[command(subcommand)]
        command: TerminalCommand,
    },
    /// List or detach local terminal clients.
    #[command(group(crate::arguments::connection_group()))]
    Client {
        #[command(subcommand)]
        command: ClientCommand,
    },
    /// List shared layouts.
    #[command(group(crate::arguments::connection_group()))]
    Layout {
        #[command(subcommand)]
        command: LayoutCommand,
    },
    /// Manage machines and import their agents; print this machine's Hub identity.
    #[command(group(crate::arguments::connection_group()))]
    Machine {
        #[command(subcommand)]
        command: yolo_tui_hub_cli::MachineCommand,
    },
    /// Inspect and control agents through the public Hub protocol.
    #[command(group(crate::arguments::connection_group()))]
    Agent {
        #[command(subcommand)]
        command: AgentCommand,
    },
    /// Start, stop, restart or inspect the local Host and Hub.
    #[command(group(crate::arguments::connection_group()))]
    Server {
        #[command(subcommand)]
        command: ServerCommand,
    },
    /// Inspect the platform snapshot or its sequenced events.
    #[command(group(crate::arguments::connection_group()))]
    Platform {
        #[command(subcommand)]
        command: PlatformCommand,
    },
    /// Read the local Hub's durable authority audit log, or merge exported logs.
    #[command(group(crate::arguments::connection_group()))]
    Audit(crate::audit::AuditArgs),
    /// Validate or start a bounded multi-agent recipe file.
    #[command(group(crate::arguments::connection_group()))]
    Recipe {
        #[command(subcommand)]
        command: RecipeCommand,
    },
    /// Read native agent conversations through the public Hub protocol.
    #[command(group(crate::arguments::connection_group()))]
    Conversation {
        #[command(subcommand)]
        action: ConversationAction,
    },
    /// Manage durable workspace metadata through the public Hub.
    #[command(group(crate::arguments::connection_group()))]
    Workspace {
        #[command(subcommand)]
        command: WorkspaceCommand,
    },
    /// Manage git worktrees through the public Hub.
    #[command(group(crate::arguments::connection_group()))]
    Worktree {
        #[command(subcommand)]
        command: WorktreeCommand,
    },
    /// Inspect the public Yolo protocol contract.
    #[command(group(crate::arguments::connection_group()))]
    Protocol {
        #[command(subcommand)]
        command: ProtocolCommand,
    },
    /// Manage trusted user-level plugins.
    #[command(group(crate::arguments::connection_group()))]
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },
    /// Check, install and activate Yolo releases.
    #[command(group(crate::arguments::connection_group()))]
    Update(UpdateArgs),
}

pub(crate) fn startup(command: Option<Command>, remote: bool) -> anyhow::Result<StartupAction> {
    match command {
        Some(Command::Attach { terminal }) => Ok(StartupAction::Attach { selector: terminal }),
        Some(Command::Terminals) => Ok(StartupAction::TerminalPicker),
        Some(Command::New { cwd, name }) => Ok(StartupAction::NewShell {
            cwd: directory(cwd, remote)?,
            name,
        }),
        None => Ok(StartupAction::Automatic {
            cwd: directory(None, remote)?,
        }),
        _ => unreachable!("noninteractive commands are dispatched before terminal startup"),
    }
}

fn directory(explicit: Option<String>, remote: bool) -> anyhow::Result<String> {
    let cwd = match explicit {
        Some(cwd) => cwd,
        None if remote => REMOTE_HOME.into(),
        None => std::env::current_dir()?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!(INVALID_CWD))?
            .into(),
    };
    anyhow::ensure!(
        (std::path::Path::new(&cwd).is_absolute() || cwd == REMOTE_HOME || cwd.starts_with("~/"))
            && !cwd.chars().any(char::is_control),
        INVALID_CWD
    );
    Ok(cwd)
}
