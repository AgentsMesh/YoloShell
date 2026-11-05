mod configuration;
mod constants;
mod listen;
mod output;
mod peer;

use anyhow::Result;
use clap::{ArgGroup, Args, Subcommand};
use constants::*;
use std::num::NonZeroU16;
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_identity::HubId;

pub use configuration::{add_change, invite_request, join_change, join_dial, listen_change};
pub use listen::Listen;

#[derive(Debug, Subcommand)]
pub enum HubCommand {
    /// Add or list federation peers on the local entry Hub.
    #[command(group(connection_group()))]
    Peer {
        #[command(subcommand)]
        command: PeerCommand,
    },
    /// Set the local Hub's peer listener address.
    #[command(group(connection_group()))]
    Listen(Listen),
}

#[derive(Debug, Subcommand)]
pub enum PeerCommand {
    /// Add a peer by Hub ID and public key; prefer invite and join for pairing.
    Add(AddPeer),
    /// Mint a pairing code; provisions 0.0.0.0:7400 if no listener is active.
    #[command(group(connection_group()))]
    Invite(InvitePeer),
    /// Join another Hub with its code and a reachable address.
    #[command(group(connection_group()))]
    Join(JoinPeer),
    /// List configured peers.
    #[command(group(connection_group()))]
    List {
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Args)]
#[command(group(connection_group()))]
pub struct AddPeer {
    /// Peer Hub ID.
    peer: HubId,
    /// Peer Ed25519 public key, 64 hexadecimal characters.
    key: String,
    /// Dial host; requires --port.
    #[arg(long, requires = "port")]
    host: Option<String>,
    /// Dial port; requires --host.
    #[arg(long, requires = "host")]
    port: Option<NonZeroU16>,
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

#[derive(Debug, Args)]
pub struct InvitePeer {
    /// Pairing code lifetime in minutes.
    #[arg(long, default_value_t = DEFAULT_INVITE_MINUTES)]
    ttl_minutes: u64,
    /// Do not let the joining Hub import this Hub's resources.
    #[arg(long)]
    no_export: bool,
    /// Import the joining Hub's resources.
    #[arg(long)]
    import_peer: bool,
    /// Print one JSON document.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
pub struct JoinPeer {
    /// Pairing code printed by the other Hub's `peer invite`.
    code: String,
    /// Host of the inviting Hub; required for yolo2_ codes.
    #[arg(long)]
    host: Option<String>,
    /// Port of the inviting Hub; defaults to the code's port, else 7400.
    #[arg(long)]
    port: Option<NonZeroU16>,
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
            PeerCommand::Invite(args) => peer::invite(socket, entry, args, deadline).await,
            PeerCommand::Join(args) => peer::join(socket, entry, args, deadline).await,
            PeerCommand::List { json } => peer::list(socket, entry, *json, deadline).await,
        },
        HubCommand::Listen(args) => listen::run(socket, entry, args, deadline).await,
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
