mod configuration;
mod constants;
mod identity;
mod output;
mod machine;

use anyhow::Result;
use clap::{ArgGroup, Args, Subcommand};
use constants::*;
use std::path::{Path, PathBuf};
use tokio::time::Instant;
use yolo_hub_identity::HubId;

pub use configuration::{import_change, pin_change, remove_change};

#[derive(Debug, Subcommand)]
pub enum MachineCommand {
    /// Import agents from another machine's Hub over SSH (one-way).
    #[command(group(connection_group()))]
    Add(AddMachine),
    /// Pin a machine with a known Hub ID and public key (advanced).
    #[command(group(connection_group()))]
    Pin(PinMachine),
    /// List configured machines.
    #[command(group(connection_group()))]
    List {
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Remove a configured machine.
    #[command(group(connection_group()))]
    Remove {
        /// Machine Hub ID.
        machine: HubId,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// List machines currently importing from this machine.
    #[command(group(connection_group()))]
    Consumers {
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Print this machine's Hub identity and peer socket.
    #[command(group(connection_group()))]
    Identity {
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Args)]
pub struct AddMachine {
    /// SSH destination of the remote machine (user@host or an ssh_config alias).
    target: String,
    /// Discover and pin the machine identity without importing its agents.
    #[arg(long)]
    pin_only: bool,
    /// Remote yolo executable used for discovery.
    #[arg(long, hide = true, default_value = REMOTE_YOLO_DEFAULT)]
    remote_yolo: String,
    /// Print one JSON document.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
pub struct PinMachine {
    /// Machine Hub ID.
    #[arg(value_parser = machine_id)]
    machine: HubId,
    /// Machine Ed25519 public key, 64 hexadecimal characters.
    key: String,
    /// SSH destination used to reach the machine (user@host or an ssh_config alias).
    #[arg(long, requires = "remote_socket")]
    ssh: Option<String>,
    /// Absolute Unix peer socket path on the remote machine.
    #[arg(long, requires = "ssh")]
    remote_socket: Option<PathBuf>,
    /// Pin the identity without importing resources from this machine.
    #[arg(long)]
    pin_only: bool,
    /// Export an origin to this machine with Observe rights; repeatable.
    #[arg(long = "export", value_name = "HUB_ID")]
    exports: Vec<HubId>,
    /// Export this machine's own origin with Observe rights.
    #[arg(long)]
    export_local: bool,
    /// Print one JSON document.
    #[arg(long)]
    json: bool,
}

// The root CLI declares the same group for its global --socket/--endpoint/--hub
// arguments; clap resolves `requires` per command, so this subtree needs its own
// group with identical ids.
fn connection_group() -> ArgGroup {
    ArgGroup::new(CONNECTION_GROUP)
        .args([SOCKET_ARGUMENT, ENDPOINT_ARGUMENT])
        .requires(HUB_ARGUMENT)
}

fn machine_id(value: &str) -> Result<HubId, String> {
    value.parse().map_err(|_| {
        format!("'{value}' is not a Hub ID; to add a machine over SSH run `yolo machine add {value}`")
    })
}

pub async fn run(
    socket: &Path,
    entry: HubId,
    command: &MachineCommand,
    deadline: Instant,
) -> Result<()> {
    match command {
        MachineCommand::Add(args) => machine::import(socket, entry, args, deadline).await,
        MachineCommand::Pin(args) => machine::pin(socket, entry, args, deadline).await,
        MachineCommand::List { json } => machine::list(socket, entry, *json, deadline).await,
        MachineCommand::Remove { machine, json } => {
            machine::remove(socket, entry, *machine, *json, deadline).await
        }
        MachineCommand::Consumers { json } => machine::consumers(socket, entry, *json, deadline).await,
        MachineCommand::Identity { json } => identity::run(socket, entry, *json, deadline).await,
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
