mod consumers;
mod import;
mod list;
mod pin;
mod remove;

use crate::constants::*;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_identity::HubId;
use yolo_hub_management_client::execute;
use yolo_hub_management_protocol::ConfigurationSnapshot;
use yolo_hub_rpc_protocol::Command as HubRpcCommand;

pub(crate) use consumers::consumers;
pub(crate) use list::list;
pub(crate) use pin::pin;
pub(crate) use import::run as import;
pub(crate) use remove::run as remove;

pub(super) async fn read(
    socket: &Path,
    entry: HubId,
    deadline: Instant,
) -> Result<ConfigurationSnapshot> {
    execute(socket, entry, HubRpcCommand::GetConfiguration {}, deadline)
        .await
        .context(HUB_CONFIGURATION_READ_FAILED)
}
