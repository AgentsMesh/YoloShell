use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "yolo", about = "Find and manage plugins for this user")]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Find and manage plugins for this user.
    Plugin(PluginOptions),
}

#[derive(Debug, Default, clap::Args)]
#[command(defer = plugin_help)]
pub struct PluginOptions {
    #[command(subcommand)]
    pub command: Option<PluginCommand>,
}

impl PluginOptions {
    pub fn into_command(self) -> PluginCommand {
        self.command.unwrap_or(PluginCommand::List {
            json: false,
            installed: false,
            registry: None,
        })
    }
}

fn plugin_help(command: clap::Command) -> clap::Command {
    let other = command
        .get_subcommands()
        .filter(|subcommand| subcommand.is_hide_set())
        .map(clap::Command::get_name)
        .collect::<Vec<_>>()
        .join(", ");
    command
        .after_help(format!(
            "More commands\n  {other}\n\nRun `yolo plugin help <command>` for details."
        ))
        .mut_args(|arg| {
            let hidden = arg.is_global_set() || arg.is_hide_set();
            arg.hide(hidden)
        })
}

#[derive(Debug, Subcommand)]
pub enum PluginCommand {
    /// Show registry plugins and what is installed locally.
    List {
        /// Only show local plugins. Works offline.
        #[arg(long, conflicts_with = "registry")]
        installed: bool,
        /// Read the catalogue from this Git registry. Omit the URL for the official registry.
        #[arg(long, value_name = "URL", num_args = 0..=1, default_missing_value = crate::install_registry::DEFAULT_REGISTRY)]
        registry: Option<String>,
        /// Print local records as JSON. Use --registry for catalogue JSON.
        #[arg(long)]
        json: bool,
    },
    /// Show one plugin's manifest, commands and grants.
    #[command(hide = true)]
    Describe {
        /// Plugin id.
        id: String,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Link a local plugin directory after reviewing its commands and digest.
    #[command(hide = true)]
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
    /// Install and enable a plugin from the registry.
    Install {
        /// Plugin id in the configured registry.
        id: String,
        /// Registry version; defaults to the registry's current version.
        #[arg(long)]
        version: Option<String>,
        /// Registry URL or name; defaults to the configured registry.
        #[arg(long)]
        registry: Option<String>,
        /// Show the verified source, commands and digest without installing.
        #[arg(long)]
        dry_run: bool,
        /// Accepted for compatibility with earlier installers.
        #[arg(long, hide = true)]
        trust: bool,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Remove a linked plugin without touching its data directory.
    #[command(hide = true)]
    Unlink {
        /// Plugin id.
        id: String,
    },
    /// Allow a linked plugin to run startup hooks and actions.
    #[command(hide = true)]
    Enable {
        /// Plugin id.
        id: String,
    },
    /// Stop a linked plugin from running hooks and actions.
    #[command(hide = true)]
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
    /// Run one action declared by the plugin.
    #[command(hide = true)]
    Action {
        /// Plugin id.
        id: String,
        /// Declared action name.
        action: String,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Show the last startup result for one plugin.
    #[command(hide = true)]
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
    #[command(hide = true)]
    DataDir {
        /// Plugin id.
        id: String,
    },
}
