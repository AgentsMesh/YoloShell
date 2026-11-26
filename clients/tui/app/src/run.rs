use crate::{
    actions,
    constants::*,
    draw, events,
    network::{self, Job},
};
use crossterm::event::EventStream;
use futures::StreamExt;
use std::time::{Duration, Instant};
use tokio::{
    sync::mpsc,
    time::{MissedTickBehavior, interval},
};
use uuid::Uuid;
use yolo_hub_client::HubEndpoint;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Command;
use yolo_tui_screen::ScreenGuard;

/// A broken or abandoned synchronized update must not freeze the local UI.
const SYNCHRONIZED_OUTPUT_HOLD_LIMIT: Duration = Duration::from_millis(250);

/// Only the active tab's panes are visible; a background tab completing its
/// update must not delay unrelated chrome.
fn synchronized_terminal(state: &crate::State) -> bool {
    state
        .tab_layout()
        .panes()
        .into_iter()
        .filter_map(|pane| pane.terminal)
        .any(|address| {
            state
                .surfaces
                .get(&address)
                .is_some_and(|surface| surface.synchronized_output())
        })
}

#[derive(Clone)]
pub struct Options {
    pub endpoint: HubEndpoint,
    pub hub: HubId,
    pub layout: Option<Uuid>,
}

pub async fn run(options: Options) -> anyhow::Result<()> {
    run_with_shutdown(options, std::future::pending()).await
}

pub async fn run_with_shutdown(
    options: Options,
    shutdown: impl std::future::Future<Output = ()>,
) -> anyhow::Result<()> {
    run_with_startup(options, shutdown, crate::StartupAction::Inventory).await
}

include!("run_loop.rs");
