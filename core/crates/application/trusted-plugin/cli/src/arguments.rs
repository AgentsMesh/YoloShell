use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "yolo", about = "Manage trusted user-level plugins")]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Manage trusted user-level plugins.
    Plugin(Options),
}

#[derive(Debug, clap::Args)]
pub struct Options {
    #[command(subcommand)]
    pub command: PluginCommand,
}

#[derive(Debug, Subcommand)]
pub enum PluginCommand {
    /// List linked plugins and their trust state.
    List {
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Show one plugin's manifest, commands and grants.
    Describe {
        /// Plugin id.
        id: String,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Link a local plugin directory after reviewing its commands and digest.
    Link {
        /// Plugin directory containing the manifest.
        dir: std::path::PathBuf,
        /// Confirm the reviewed commands and digest; without it, only prints the review.
        #[arg(long)]
        trust: bool,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Install a registry plugin after reviewing its source digest.
    Install {
        /// Plugin id in the configured registry.
        id: String,
        /// Registry version; defaults to the registry's current version.
        #[arg(long)]
        version: Option<String>,
        /// Registry URL or name; defaults to the configured registry.
        #[arg(long)]
        registry: Option<String>,
        /// Confirm the reviewed source commands and digest; without it, only prints the review.
        #[arg(long)]
        trust: bool,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Remove a linked plugin without touching its data directory.
    Unlink {
        /// Plugin id.
        id: String,
    },
    /// Allow a linked plugin to run startup hooks and actions.
    Enable {
        /// Plugin id.
        id: String,
    },
    /// Stop a linked plugin from running hooks and actions.
    Disable {
        /// Plugin id.
        id: String,
    },
    /// Run the startup hooks of all enabled plugins.
    Startup {
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Invoke one declared plugin action with a JSON argument.
    Action {
        /// Plugin id.
        id: String,
        /// Declared action name.
        action: String,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Show the last startup or action result for one plugin.
    Status {
        /// Plugin id.
        id: String,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Print one plugin's bounded startup and action log.
    Log {
        /// Plugin id.
        id: String,
    },
    /// Print one plugin's writable data directory.
    DataDir {
        /// Plugin id.
        id: String,
    },
}
