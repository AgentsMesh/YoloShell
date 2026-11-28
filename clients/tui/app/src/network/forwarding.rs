//! Channel-to-update forwarding for the interactive network worker.

use crate::{constants::QUEUE_CAPACITY, network::Update};
use tokio::sync::mpsc;
use yolo_hub_client_channels::{CatalogEvent, ControlEvent, Endpoint};

pub async fn controls(mut events: mpsc::Receiver<ControlEvent>, updates: mpsc::Sender<Update>) {
    while let Some(event) = events.recv().await {
        let _ = updates.send(Update::Control(event)).await;
    }
}

pub async fn catalogs(mut events: mpsc::Receiver<CatalogEvent>, updates: mpsc::Sender<Update>) {
    while let Some(event) = events.recv().await {
        let _ = updates.send(Update::Catalog(event)).await;
    }
}

/// Platform events only invalidate the catalogue; the snapshot is reconciled
/// through the normal fenced refresh path. A Hub without the subscription
/// method ends the watcher and leaves the periodic refresh as the fallback.
pub async fn platforms(endpoint: Endpoint, updates: mpsc::Sender<Update>, subscribe: bool) {
    let (wake, mut wakes) = mpsc::channel(QUEUE_CAPACITY);
    if subscribe {
        tokio::spawn(yolo_hub_client_channels::watch_platform_events(
            endpoint, wake,
        ));
    } else {
        drop(wake);
    }
    while let Some(event) = wakes.recv().await {
        let _ = updates.send(Update::PlatformEvents(event)).await;
    }
}
