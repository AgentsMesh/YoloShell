use crate::configuration::remove_change;
use crate::constants::*;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_identity::HubId;
use yolo_hub_management_client::execute;
use yolo_hub_rpc_protocol::Command as HubRpcCommand;

pub(crate) async fn run(
    socket: &Path,
    entry: HubId,
    hub: HubId,
    json: bool,
    deadline: Instant,
) -> Result<()> {
    let snapshot = super::read(socket, entry, deadline).await?;
    let change = remove_change(&snapshot, hub)?;
    let updated = execute(socket, entry, HubRpcCommand::Configure { change }, deadline)
        .await
        .context(HUB_CONFIGURATION_WRITE_FAILED)?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "removed": hub.to_string(),
                "applied": updated.applied,
            })
        );
    } else {
        println!("removed peer {hub}");
        if !updated.applied {
            println!("configuration is persisted but not active");
        }
    }
    Ok(())
}
