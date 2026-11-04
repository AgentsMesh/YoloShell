use crate::constants::*;
use crate::platform::PlatformCommand;
use crate::recipe::RecipeCommand;
use clap::Subcommand;
use yolo_tui_app::StartupAction;

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Open a new tab with a normal shell. Run agents directly in that shell.
    #[command(group(crate::arguments::connection_group()))]
    New {
        #[arg(long)]
        cwd: Option<String>,
        /// Label for the terminal; selectors need the label to be unambiguous.
        #[arg(long)]
        name: Option<String>,
    },
    /// Attach an existing terminal by ID, address, or unique name; never creates a shell.
    #[command(group(crate::arguments::connection_group()))]
    Attach { terminal: Option<String> },
    /// Disconnect a local terminal client; keep its shells and agents running.
    #[command(group(crate::arguments::connection_group()))]
    Detach { client: Option<uuid::Uuid> },
    /// List terminals visible from the selected Hub.
    #[command(group(crate::arguments::connection_group()))]
    List {
        #[arg(long)]
        json: bool,
        /// Include archived terminals from history.
        #[arg(long)]
        all: bool,
    },
    /// List local terminal clients connected to the selected Hub.
    #[command(group(crate::arguments::connection_group()))]
    Clients {
        #[arg(long)]
        json: bool,
    },
    /// Archive an exited terminal; its history keeps the row.
    #[command(group(crate::arguments::connection_group()))]
    Archive {
        /// Terminal id, address, or unique name.
        terminal: String,
        #[arg(long)]
        json: bool,
    },
    /// End a terminal and every program running in it.
    #[command(group(crate::arguments::connection_group()))]
    Terminate {
        /// Terminal id, address, or unique name.
        terminal: String,
        /// Kill programs that do not exit after a graceful request.
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    /// List shared layout IDs; use --layout to open one in an independent client.
    #[command(group(crate::arguments::connection_group()))]
    Layouts {
        #[arg(long)]
        json: bool,
    },
    /// Open the terminal picker to attach an existing shell without creating one.
    #[command(name = TERMINALS_COMMAND, group(crate::arguments::connection_group()))]
    Terminals,
    /// Print the visible screen text of a terminal (non-authoritative).
    #[command(group(crate::arguments::connection_group()))]
    Screen {
        /// Terminal id, address, or unique name.
        terminal: String,
        #[arg(long)]
        json: bool,
    },
    /// Inspect the platform snapshot or its sequenced events.
    #[command(group(crate::arguments::connection_group()))]
    Platform {
        #[command(subcommand)]
        command: PlatformCommand,
    },
    /// Add or list federation peers on the local entry Hub.
    #[command(group(crate::arguments::connection_group()))]
    Hub {
        #[command(subcommand)]
        command: yolo_tui_hub_cli::HubCommand,
    },
    /// Show installed and running local service status.
    #[command(group(crate::arguments::connection_group()))]
    Status(yolo_tui_service::FormatOptions),
    /// Print installed and running local service versions.
    #[command(group(crate::arguments::connection_group()))]
    Version(yolo_tui_service::FormatOptions),
    /// Start the local Host and Hub.
    #[command(group(crate::arguments::connection_group()))]
    Start(yolo_tui_service::FormatOptions),
    /// Stop the local Host and Hub; terminals and PTYs keep running.
    #[command(group(crate::arguments::connection_group()))]
    Stop(yolo_tui_service::FormatOptions),
    /// Restart the local Host and Hub.
    #[command(group(crate::arguments::connection_group()))]
    Restart(yolo_tui_service::FormatOptions),
    /// Read the local Hub's durable authority audit log, or merge exported logs.
    #[command(group(crate::arguments::connection_group()))]
    Audit {
        /// Only records after this sequence.
        #[arg(long)]
        after: Option<u64>,
        /// Maximum records to print.
        #[arg(long, default_value_t = crate::constants::DEFAULT_AUDIT_LIMIT)]
        limit: u16,
        /// Merge one or more audit JSONL files instead of reading the profile.
        #[arg(long = "merge", value_name = "FILE")]
        merge: Vec<std::path::PathBuf>,
        /// Trusted Hub identity for an authenticated merge: HUB_ID=PUBLIC_KEY.
        #[arg(long, value_name = "HUB_ID=PUBLIC_KEY")]
        trust: Vec<String>,
        /// Print this profile's Hub signing identity and exit.
        #[arg(long)]
        identity: bool,
        /// Pull signed audit lines from this peer through the local entry Hub.
        #[arg(long, value_name = "HUB_ID")]
        pull: Option<yolo_hub_identity::HubId>,
        /// Write pulled lines to this file instead of stdout.
        #[arg(long, value_name = "FILE", requires = "pull")]
        output: Option<std::path::PathBuf>,
        #[arg(long)]
        json: bool,
    },
    /// Validate or start a bounded multi-agent recipe file.
    #[command(group(crate::arguments::connection_group()))]
    Recipe {
        #[command(subcommand)]
        command: RecipeCommand,
    },
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
