use crate::constants::{NO_LOCAL_HUB, RPC_TIMEOUT, SOCKET_PAIR};
use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use tokio::time::Instant;
use yolo_hub_client::LocalClient;
use yolo_hub_identity::HubId;
use yolo_hub_state::HubState;
use yolo_local_profile::LocalProfile;

pub struct Connection {
    pub socket: Option<PathBuf>,
    pub hub: Option<HubId>,
    pub local_profile: Option<PathBuf>,
}

pub async fn connect(connection: &Connection) -> Result<LocalClient> {
    let deadline = Instant::now() + RPC_TIMEOUT;
    match (&connection.socket, connection.hub) {
        (Some(socket), Some(hub)) => Ok(LocalClient::connect_local(socket, hub, deadline).await?),
        (None, None) => {
            let profile = match &connection.local_profile {
                Some(path) => LocalProfile::load(path)?,
                None => yolo_local_profile::current()?,
            };
            let hub = HubState::inspect_identity(&profile.state, &profile.host_home)?
                .context(NO_LOCAL_HUB)?;
            let socket = HubState::socket_path_for(&profile.state)?;
            Ok(LocalClient::connect_local(&socket, hub, deadline).await?)
        }
        _ => bail!(SOCKET_PAIR),
    }
}
