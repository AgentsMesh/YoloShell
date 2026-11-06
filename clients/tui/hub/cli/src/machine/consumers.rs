use crate::constants::*;
use anyhow::{Result, bail};
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_identity::HubId;
use yolo_hub_management_client::execute_reply;
use yolo_hub_rpc_protocol::{Command, Reply};

pub(crate) async fn consumers(
    socket: &Path,
    entry: HubId,
    json: bool,
    deadline: Instant,
) -> Result<()> {
    let reply = execute_reply(socket, entry, Command::ListConsumers {}, deadline).await?;
    let Reply::Consumers { consumers } = reply else {
        bail!(UNEXPECTED_REPLY);
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&consumers)?);
        return Ok(());
    }
    if consumers.is_empty() {
        println!("{MACHINE_CONSUMERS_EMPTY}");
        return Ok(());
    }
    println!("{MACHINE_CONSUMERS_HEADER}");
    for consumer in consumers {
        println!(
            "{}\t{}\t{}",
            consumer.hub, consumer.links, consumer.connected_since_ms
        );
    }
    Ok(())
}
