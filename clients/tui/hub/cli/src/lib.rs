mod configuration;
mod constants;
mod identity;
mod output;
mod peer;

use anyhow::Result;
use clap::{ArgGroup, Args, Subcommand};
use constants::*;
use std::path::{Path, PathBuf};
use tokio::time::Instant;
use yolo_hub_identity::HubId;

pub use configuration::{add_change, join_change, remove_change};

#[derive(Debug, Subcommand)]
pub enum HubCommand {
    /// Add, list or join federation peers on the local entry Hub.
    #[command(group(connection_group()))]
    Peer {
        #[command(subcommand)]
        command: PeerCommand,
    },
    /// Print this Hub's public identity and peer socket.
    #[command(group(connection_group()))]
    Identity {
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum PeerCommand {
    /// Add a peer explicitly with its Hub identity and SSH dial.
    Add(AddPeer),
    /// List configured peers.
    #[command(group(connection_group()))]
    List {
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Join a provider Hub over the user's existing SSH access.
    Join(JoinPeer),
    /// Remove a configured peer.
    #[command(group(connection_group()))]
    Remove {
        /// Peer Hub ID.
        peer: HubId,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Args)]
#[command(group(connection_group()))]
pub struct JoinPeer {
    /// SSH destination of the provider Hub (user@host or an ssh_config alias).
    target: String,
    /// Do not import resources from the provider.
    #[arg(long)]
    no_import: bool,
    /// Remote yolo executable used for discovery.
    #[arg(long, hide = true, default_value = JOIN_REMOTE_YOLO)]
    remote_yolo: String,
    /// Print one JSON document.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
#[command(group(connection_group()))]
pub struct AddPeer {
    /// Peer Hub ID.
    peer: HubId,
    /// Peer Ed25519 public key, 64 hexadecimal characters.
    key: String,
    /// SSH destination used to reach the peer Hub (user@host or an ssh_config alias).
    #[arg(long, requires = "remote_socket")]
    ssh: Option<String>,
    /// Absolute Unix peer socket path on the remote Hub.
    #[arg(long, requires = "ssh")]
    remote_socket: Option<PathBuf>,
    /// Do not import resources from this peer.
    #[arg(long)]
    no_import: bool,
    /// Export an origin to this peer with Observe rights; repeatable.
    #[arg(long = "export", value_name = "HUB_ID")]
    exports: Vec<HubId>,
    /// Export this Hub's own origin with Observe rights.
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

pub async fn run(
    socket: &Path,
    entry: HubId,
    command: &HubCommand,
    deadline: Instant,
) -> Result<()> {
    match command {
        HubCommand::Peer { command } => match command {
            PeerCommand::Add(args) => peer::add(socket, entry, args, deadline).await,
            PeerCommand::List { json } => peer::list(socket, entry, *json, deadline).await,
            PeerCommand::Join(args) => peer::join(socket, entry, args, deadline).await,
            PeerCommand::Remove { peer, json } => {
                peer::remove(socket, entry, *peer, *json, deadline).await
            }
        },
        HubCommand::Identity { json } => identity::run(socket, entry, *json, deadline).await,
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
