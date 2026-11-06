use crate::constants::*;
use crate::output;
use crate::PinMachine;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_identity::HubId;
use yolo_hub_management_client::execute;
use yolo_hub_rpc_protocol::Command as HubRpcCommand;

pub(crate) async fn pin(
    socket: &Path,
    entry: HubId,
    args: &PinMachine,
    deadline: Instant,
) -> Result<()> {
    let snapshot = super::read(socket, entry, deadline).await?;
    let change = crate::pin_change(&snapshot, args)?;
    let updated = execute(socket, entry, HubRpcCommand::Configure { change }, deadline)
        .await
        .context(HUB_CONFIGURATION_WRITE_FAILED)?;
    if args.json {
        println!("{}", output::machine_json(&updated, args.machine)?);
    } else {
        println!(
            "pinned machine {} at revision {}",
            args.machine, updated.state.revision
        );
        if !updated.applied {
            println!("configuration is persisted but not active");
        }
    }
    Ok(())
}

