use clap::{Args, Parser, Subcommand};
use serde::Deserialize;
use std::{net::SocketAddr, path::PathBuf};
use yolo_hub_client_access_protocol::ClientIdentity;
use yolo_hub_identity::HubId;
use yolo_web_gateway::{DEFAULT_MAX_CONNECTIONS, DEFAULT_SESSION_LIFETIME};

const DEFAULT_LISTEN: &str = "127.0.0.1:0";

#[derive(Parser)]
#[command(name = "yolo", about = "Yolo authenticated Web gateway")]
pub(crate) struct Arguments {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    Web(WebOptions),
}

#[derive(Args)]
pub(crate) struct WebOptions {
    /// Private scoped Hub profile: socket, pinned Hub key, enrolled client identity.
    #[arg(long)]
    pub profile: PathBuf,
    /// Enrolled Ed25519 client key; never the Hub or federation private key.
    #[arg(long)]
    pub client_key: PathBuf,
    /// Private file containing a random 32-byte token as lowercase hexadecimal.
    #[arg(long)]
    pub token_file: PathBuf,
    /// Exact permitted browser Origin; repeat for multiple frontend origins.
    #[arg(long, required = true)]
    pub origin: Vec<String>,
    /// Loopback bind only; use a secure tunnel to access it remotely.
    #[arg(long, default_value = DEFAULT_LISTEN)]
    pub listen: SocketAddr,
    #[arg(long, default_value_t = DEFAULT_SESSION_LIFETIME.as_secs())]
    pub session_seconds: u64,
    #[arg(long, default_value_t = DEFAULT_MAX_CONNECTIONS)]
    pub max_connections: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Profile {
    pub socket: PathBuf,
    pub hub_id: HubId,
    pub hub_public_key: [u8; 32],
    pub client: ClientIdentity,
}
