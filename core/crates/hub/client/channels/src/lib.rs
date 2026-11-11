//! Independent ordered control, lease maintenance, catalogue, lifecycle and query owners.
mod admission;
mod catalog;
mod connection_liveness;
mod constants;
mod control;
mod error;
mod input_batch;
mod input_completion;
mod input_stream;
pub use input_stream::InputStream;
mod lease_deadline;
mod maintenance;
mod model;
mod operations;
mod platform_events;
mod queries;
mod request_validity;
pub use request_validity::{RequestScope, RequestValidity};

pub use constants::{
    CONTROL_KEEPALIVE_INTERVAL, MAX_LIFECYCLE_CHANNELS, MAX_QUERY_CHANNELS, RPC_TIMEOUT,
};
pub use error::ControlError;
pub use model::{CatalogEvent, ControlEvent, ControlRequest, Endpoint};
pub use platform_events::{PlatformEventWake, watch_platform_events};

use crate::constants::INPUT_DISPATCH_CAPACITY;
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

pub async fn run(
    endpoint: Endpoint,
    commands: mpsc::Receiver<ControlRequest>,
    maintenance: mpsc::Receiver<ControlRequest>,
    refresh: watch::Receiver<Option<Uuid>>,
    control_updates: mpsc::Sender<ControlEvent>,
    catalog_updates: mpsc::Sender<CatalogEvent>,
) {
    let (generation, current) = watch::channel(None);
    let (batched_commands, batched_receiver) = mpsc::channel(INPUT_DISPATCH_CAPACITY);
    tokio::join!(
        control::run(
            endpoint.clone(),
            batched_receiver,
            control_updates.clone(),
            generation
        ),
        input_batch::run(commands, batched_commands),
        maintenance::run(endpoint.clone(), maintenance, control_updates, current),
        catalog::run(endpoint, refresh, catalog_updates),
    );
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
