use crate::constants::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::{net::UnixStream, sync::Semaphore, task::JoinSet, time::Instant};
use yolo_hub_client_access_registry::AccessRegistry;
use yolo_hub_client_transport::{ClientIo, accept};
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_state::HubState;

pub async fn serve(
    stream: UnixStream,
    key: Arc<russh::keys::PrivateKey>,
    registry: AccessRegistry,
    dispatcher: Arc<dyn Dispatcher>,
    state: Arc<HubState>,
    capacity: Arc<Semaphore>,
    audit: Option<Arc<yolo_hub_audit::AuditLog>>,
) -> Result<()> {
    let (connection, mut incoming) = accept(
        ClientIo::from_unix(stream)?,
        key,
        registry,
        Instant::now() + AUTHENTICATION_TIMEOUT,
    )
    .await?;
    let mut channels = JoinSet::new();
    let result = loop {
        tokio::select! {
            biased;
            _ = connection.closed() => break Ok(()),
            completed = channels.join_next(), if !channels.is_empty() => {
                if !matches!(completed, Some(Ok(Ok(())))) { tracing::debug!(CHANNEL_FAILED); }
            },
            channel = incoming.recv() => {
                let Some(channel) = channel else { break Ok(()); };
                state.verify()?;
                let Ok(permit) = capacity.clone().try_acquire_owned() else { continue; };
                let dispatcher = dispatcher.clone();
                let state = state.clone();
                let audit = audit.clone();
                channels.spawn(async move {
                    let _permit = permit;
                    yolo_hub_server::serve_admitted_audited(
                        channel.stream,
                        dispatcher,
                        state,
                        channel.admission,
                        audit,
                    )
                    .await
                });
            },
        }
    };
    channels.abort_all();
    while channels.join_next().await.is_some() {}
    let _ = connection.close(Instant::now() + CLOSE_TIMEOUT).await;
    result
}
