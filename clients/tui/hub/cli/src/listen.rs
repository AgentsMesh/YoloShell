use crate::constants::*;
use crate::output;
use anyhow::{Context, Result};
use clap::Args;
use std::net::SocketAddr;
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_identity::HubId;
use yolo_hub_management_client::execute;
use yolo_hub_rpc_protocol::Command as HubRpcCommand;

#[derive(Debug, Args)]
pub struct Listen {
    /// Address the peer listener should bind, such as 0.0.0.0:7400 or 127.0.0.1:0.
    pub address: SocketAddr,
    /// Print one JSON document.
    #[arg(long)]
    pub json: bool,
}

pub(crate) async fn run(
    socket: &Path,
    entry: HubId,
    args: &Listen,
    deadline: Instant,
) -> Result<()> {
    let snapshot = execute(socket, entry, HubRpcCommand::GetConfiguration {}, deadline)
        .await
        .context(HUB_CONFIGURATION_READ_FAILED)?;
    let change = crate::listen_change(&snapshot, args.address)?;
    let updated = execute(socket, entry, HubRpcCommand::Configure { change }, deadline)
        .await
        .context(HUB_CONFIGURATION_WRITE_FAILED)?;
    if args.json {
        println!("{}", output::listen_json(&updated)?);
    } else if updated.applied {
        let address = updated
            .listen_address
            .map_or_else(|| args.address.to_string(), |address| address.to_string());
        println!(
            "listening on {address} at revision {}",
            updated.state.revision
        );
    } else {
        println!(
            "listener {} is persisted at revision {} but not active",
            args.address, updated.state.revision
        );
    }
    Ok(())
}
