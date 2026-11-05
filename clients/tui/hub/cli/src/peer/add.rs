use crate::constants::*;
use crate::output;
use crate::AddPeer;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_identity::HubId;
use yolo_hub_management_client::execute;
use yolo_hub_rpc_protocol::Command as HubRpcCommand;

pub(crate) async fn add(
    socket: &Path,
    entry: HubId,
    args: &AddPeer,
    deadline: Instant,
) -> Result<()> {
    let snapshot = super::read(socket, entry, deadline).await?;
    let change = crate::add_change(&snapshot, args)?;
    let updated = execute(socket, entry, HubRpcCommand::Configure { change }, deadline)
        .await
        .context(HUB_CONFIGURATION_WRITE_FAILED)?;
    if args.json {
        println!("{}", output::add_json(&updated, args.peer)?);
    } else {
        println!(
            "added peer {} at revision {}",
            args.peer, updated.state.revision
        );
        if !updated.applied {
            println!("configuration is persisted but not active");
        }
    }
    Ok(())
}

pub(crate) async fn list(
    socket: &Path,
    entry: HubId,
    json: bool,
    deadline: Instant,
) -> Result<()> {
    let snapshot = super::read(socket, entry, deadline).await?;
    if json {
        println!("{}", output::list_json(&snapshot)?);
        return Ok(());
    }
    let peers = &snapshot.state.configuration.peers;
    if peers.is_empty() {
        println!("{PEER_LIST_EMPTY}");
        return Ok(());
    }
    println!("{HUB_PEER_HEADER}");
    for peer in peers {
        let dial = peer.dial.as_ref().map_or_else(
            || "-".into(),
            |dial| {
                format!(
                    "{}:{}",
                    dial.profile,
                    dial.socket.display()
                )
            },
        );
        let imports = if peer.consume { "yes" } else { "no" };
        let exports = peer
            .exports
            .iter()
            .map(|policy| policy.origin.to_string())
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{}\t{}\t{}\t{}",
            peer.identity.hub(),
            dial,
            imports,
            if exports.is_empty() {
                "-".into()
            } else {
                exports
            }
        );
    }
    println!("{PEER_LIST_LEGEND}");
    Ok(())
}
