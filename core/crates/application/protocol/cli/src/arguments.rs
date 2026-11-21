use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "yolo", about = "Inspect the public Yolo protocol contract")]
pub(crate) struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Describe this image's public Hub contract; live capabilities may be a subset.
    Protocol {
        #[command(subcommand)]
        command: ProtocolCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum ProtocolCommand {
    /// Describe the public Hub contract. Output is machine-readable JSON.
    Describe {
        /// Print machine-readable JSON (this command's only output format).
        #[arg(long)]
        json: bool,
    },
}
