use super::support::deadline;
use std::path::Path;
use yolo_hub_client::Client;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};

pub async fn call(socket: &Path, entry: HubId, destination: HubId, command: Command) -> Reply {
    let mut client = Client::connect_local(socket, entry, deadline()).await.unwrap();
    client.call(destination, command, deadline()).await.unwrap()
}
