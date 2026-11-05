mod add;
mod invite;
mod join;

use crate::constants::*;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_identity::HubId;
use yolo_hub_management_client::execute;
use yolo_hub_management_protocol::ConfigurationSnapshot;
use yolo_hub_rpc_protocol::Command as HubRpcCommand;

pub(crate) use add::{add, list};
pub(crate) use invite::{expires_in_text, invite, provisioned_listener};
pub(crate) use join::join;

pub(super) async fn read(
    socket: &Path,
    entry: HubId,
    deadline: Instant,
) -> Result<ConfigurationSnapshot> {
    execute(socket, entry, HubRpcCommand::GetConfiguration {}, deadline)
        .await
        .context(HUB_CONFIGURATION_READ_FAILED)
}

pub(super) fn wall_clock_ms() -> Result<u64> {
    Ok(u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?)
}
