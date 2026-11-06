use crate::{
    AcceptedConnection, ClientIo, IncomingChannel,
    constants::*,
    server::{self, ServerHandler},
    time::now_ms,
};
use anyhow::{Context, Result, ensure};
use russh::keys::PrivateKey;
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, Semaphore, mpsc},
    time::{Instant, timeout_at},
};
use uuid::Uuid;
use yolo_hub_client_access_registry::AccessRegistry;

pub async fn accept(
    transport: ClientIo,
    key: Arc<PrivateKey>,
    registry: AccessRegistry,
    deadline: Instant,
) -> Result<(AcceptedConnection, mpsc::Receiver<IncomingChannel>)> {
    ensure!(key.public_key().key_data().ed25519().is_some(), INVALID_KEY);
    let (stream, transport) = transport.into_parts();
    let (incoming, receiver) = mpsc::channel(MAX_QUEUED_CHANNELS);
    let (authenticated, mut authenticated_client) = mpsc::channel(AUTH_EVENT_CAPACITY);
    let connection_id = Uuid::new_v4();
    let handler = ServerHandler {
        registry: Some(registry),
        authenticated,
        admission: None,
        connection_id,
        incoming,
        capacity: Arc::new(Semaphore::new(MAX_CLIENT_CHANNELS)),
        transport: transport.0.clone(),
    };
    let running = timeout_at(
        deadline,
        russh::server::run_stream(server::config(key), stream, handler),
    )
    .await??;
    let admission = timeout_at(deadline, authenticated_client.recv())
        .await?
        .context(AUTH_FAILED)?;
    let now = now_ms()?;
    ensure!(admission.is_current(now), AUTH_FAILED);
    let validity = admission.clone();
    let transport_state = transport.0.clone();
    let revocation = tokio::spawn(async move {
        loop {
            let Ok(now) = now_ms() else {
                break;
            };
            if !validity.is_current(now) {
                break;
            }
            let remaining =
                Duration::from_millis(validity.grant().expires_at_ms.saturating_sub(now));
            tokio::select! {
                _ = validity.revoked() => break,
                _ = tokio::time::sleep(remaining.min(EXPIRY_RECHECK_INTERVAL)) => {},
            }
        }
        transport_state.close();
    });
    Ok((
        AcceptedConnection {
            admission,
            connection_id,
            transport,
            running: Mutex::new(Some(running)),
            revocation,
        },
        receiver,
    ))
}
