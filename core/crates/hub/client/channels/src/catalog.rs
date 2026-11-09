use crate::{
    CatalogEvent, Endpoint,
    constants::{CONTROL_KEEPALIVE_INTERVAL, RPC_TIMEOUT},
};
use tokio::{
    sync::{mpsc, watch},
    time::{Instant, sleep},
};
use uuid::Uuid;
use yolo_hub_catalog_client::{Catalog, CatalogError};
use yolo_hub_client::EndpointClient;

pub(crate) async fn run(
    endpoint: Endpoint,
    mut refresh: watch::Receiver<Option<Uuid>>,
    updates: mpsc::Sender<CatalogEvent>,
) {
    let mut client = None;
    let mut requested = *refresh.borrow_and_update();
    let mut idle = Box::pin(sleep(CONTROL_KEEPALIVE_INTERVAL));
    loop {
        if refresh.has_changed().is_err() || updates.is_closed() {
            return;
        }
        let Some(request_id) = requested else {
            tokio::select! {
                biased;
                _ = &mut idle, if client.is_some() => client = None,
                changed = refresh.changed() => {
                    if changed.is_err() { return; }
                    requested = *refresh.borrow_and_update();
                }
                _ = updates.closed() => return,
            }
            continue;
        };
        // A blocked consumer or machine suspension may outlive the retention
        // deadline before the timer was polled. Never reuse that cached socket.
        if Instant::now() >= idle.deadline() {
            client = None;
        }
        let result = {
            let load = load(&endpoint, &mut client);
            tokio::pin!(load);
            tokio::select! {
                biased;
                changed = refresh.changed() => {
                    if changed.is_err() { return; }
                    None
                }
                _ = updates.closed() => return,
                result = &mut load => Some(result),
            }
        };
        let Some(result) = result else {
            // A replacement never reuses a partially read catalogue frame.
            client = None;
            requested = *refresh.borrow_and_update();
            continue;
        };
        idle.as_mut()
            .reset(Instant::now() + CONTROL_KEEPALIVE_INTERVAL);
        let event = CatalogEvent { request_id, result };
        tokio::select! {
            biased;
            changed = refresh.changed() => {
                if changed.is_err() { return; }
                requested = *refresh.borrow_and_update();
            }
            sent = updates.send(event) => {
                if sent.is_err() { return; }
                requested = None;
            }
        }
    }
}

async fn load(
    endpoint: &Endpoint,
    client: &mut Option<EndpointClient>,
) -> Result<Catalog, CatalogError> {
    let deadline = Instant::now() + RPC_TIMEOUT;
    if client.as_ref().is_none_or(|client| !client.is_usable()) {
        *client = None;
        *client = Some(endpoint.entry.connect(endpoint.hub, deadline).await?);
    }
    let current = client
        .as_mut()
        .ok_or(yolo_hub_client::ClientError::Poisoned)?;
    // The TUI is also the recovery surface: archived records must remain
    // observable so their bindings can be retired and history stays truthful.
    yolo_hub_catalog_client::read_with_archived(current, deadline, true).await
}
