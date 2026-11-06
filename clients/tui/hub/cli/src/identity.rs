use crate::constants::*;
use crate::output;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::time::Instant;
use yolo_hub_identity::HubId;

pub(crate) async fn run(
    socket: &Path,
    entry: HubId,
    json: bool,
    deadline: Instant,
) -> Result<()> {
    let snapshot = crate::machine::read(socket, entry, deadline).await?;
    let identity = snapshot.state.identity;
    let peer_socket = snapshot.peer_socket.context(PEER_SOCKET_UNAVAILABLE)?;
    if json {
        println!("{}", output::identity_json(identity, &peer_socket));
    } else {
        println!("HUB ID\t{}", identity.hub());
        println!("PUBLIC KEY\t{}", output::key_text(identity));
        println!("LINK SOCKET\t{}", peer_socket.display());
    }
    Ok(())
}
