use anyhow::{Result, ensure};
use russh::keys::PrivateKey;
use std::{path::PathBuf, sync::Arc};
use tokio::{
    net::UnixStream,
    time::{Instant, timeout_at},
};
use yolo_hub_client::Client;
use yolo_hub_client_access_protocol::ClientIdentity;
use yolo_hub_client_transport::{ClientConnection, ClientIo, ClientStream, HubPin};

/// An enrolled scoped client, never a same-UID management endpoint.
pub struct ScopedHub {
    socket: PathBuf,
    pub(crate) pin: HubPin,
    pub(crate) identity: ClientIdentity,
    key: Arc<PrivateKey>,
}

impl ScopedHub {
    pub fn new(
        socket: PathBuf,
        pin: HubPin,
        identity: ClientIdentity,
        key: Arc<PrivateKey>,
    ) -> Result<Self> {
        ensure!(socket.is_absolute(), "scoped Hub socket must be absolute");
        identity.validate()?;
        ensure!(
            key.public_key()
                .key_data()
                .ed25519()
                .is_some_and(|public| public.0 == identity.public_key),
            "web client key does not match enrolled identity"
        );
        Ok(Self {
            socket,
            pin,
            identity,
            key,
        })
    }

    pub(crate) async fn open(
        &self,
        deadline: Instant,
    ) -> Result<(Client<ClientStream>, ClientConnection)> {
        let socket = timeout_at(deadline, UnixStream::connect(&self.socket)).await??;
        let connection = yolo_hub_client_transport::connect(
            ClientIo::from_unix(socket)?,
            self.key.clone(),
            self.pin,
            deadline,
        )
        .await?;
        let stream = connection.open_channel(deadline).await?;
        let client = Client::open(stream, self.pin.hub_id, deadline).await?;
        Ok((client, connection))
    }
}
