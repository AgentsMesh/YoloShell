use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "yolo", about = "Manage trusted user-level plugins")]
pub struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Plugin(Options),
}

#[derive(Debug, clap::Args)]
pub struct Options {
    #[command(subcommand)]
    pub command: PluginCommand,
}

#[derive(Debug, Subcommand)]
pub enum PluginCommand {
    List {
        #[arg(long)]
        json: bool,
    },
    Describe {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Link {
        dir: std::path::PathBuf,
        #[arg(long)]
        trust: bool,
        #[arg(long)]
        json: bool,
    },
    Unlink {
        id: String,
    },
    Enable {
        id: String,
    },
    Disable {
        id: String,
    },
    Startup {
        #[arg(long)]
        json: bool,
    },
    Action {
        id: String,
        action: String,
        #[arg(long)]
        json: bool,
    },
    Status {
        id: String,
        #[arg(long)]
        json: bool,
    },
    Log {
        id: String,
    },
    DataDir {
        id: String,
    },
}
