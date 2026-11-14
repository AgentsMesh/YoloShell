use crate::{
    ClientConnection, ClientIo, HubPin,
    client::{self, ClientHandler},
    constants::*,
};
use anyhow::{Result, ensure};
use russh::keys::{PrivateKey, PrivateKeyWithHashAlg};
use std::sync::Arc;
use tokio::{
    sync::{Mutex, Semaphore},
    time::{Instant, timeout_at},
};

pub async fn connect(
    transport: ClientIo,
    key: Arc<PrivateKey>,
    pin: HubPin,
    deadline: Instant,
) -> Result<ClientConnection> {
    ensure!(key.public_key().key_data().ed25519().is_some(), INVALID_KEY);
    let (stream, transport) = transport.into_parts();
    let handler = ClientHandler {
        pin,
        transport: transport.0.clone(),
    };
    let mut handle = timeout_at(
        deadline,
        russh::client::connect_stream(client::config(), stream, handler),
    )
    .await??;
    let result = timeout_at(
        deadline,
        handle.authenticate_publickey(PUBLIC_CLIENT_USER, PrivateKeyWithHashAlg::new(key, None)),
    )
    .await??;
    ensure!(result.success(), AUTH_FAILED);
    Ok(ClientConnection {
        pin,
        handle: Mutex::new(Some(handle)),
        transport,
        capacity: Arc::new(Semaphore::new(MAX_CLIENT_CHANNELS)),
    })
}
