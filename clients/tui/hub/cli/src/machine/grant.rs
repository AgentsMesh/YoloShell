use crate::{GrantMachine, constants::*, output};
use anyhow::{Context, Result};
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_identity::HubId;
use yolo_hub_management_client::execute;
use yolo_hub_rpc_protocol::Command as HubRpcCommand;

pub(crate) async fn grant(
    socket: &Path,
    entry: HubId,
    args: &GrantMachine,
    deadline: Instant,
) -> Result<()> {
    let snapshot = super::read(socket, entry, deadline).await?;
    let change = crate::grant_change(&snapshot, args)?;
    let updated = execute(socket, entry, HubRpcCommand::Configure { change }, deadline)
        .await
        .context(HUB_CONFIGURATION_WRITE_FAILED)?;
    if args.json {
        println!("{}", output::machine_json(&updated, args.consumer)?);
    } else {
        println!(
            "granted control access for {} at revision {}",
            args.consumer, updated.state.revision
        );
        if !updated.applied {
            println!("configuration is persisted but not active");
        }
    }
    Ok(())
}
