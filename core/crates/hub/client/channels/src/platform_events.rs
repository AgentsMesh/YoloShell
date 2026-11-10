//! Platform event subscription used by interactive clients to wake catalogue
//! reconciliation. Delivery is a bounded long-poll; a lost cursor is reported
//! so the caller reconciles with a snapshot instead of replaying history.

use crate::{Endpoint, constants::RPC_TIMEOUT};
use std::collections::BTreeSet;
use std::time::Duration;
use tokio::{
    sync::mpsc,
    task::JoinSet,
    time::{Instant, sleep},
};
use yolo_hub_client::EndpointClient;
use yolo_hub_events_protocol::{
    MAX_PLATFORM_EVENT_WAIT_MS, PLATFORM_EVENT_SCHEMA_VERSION, PlatformEventCursor,
    PlatformEventKind, SubscribeEventsFields, SubscribeEventsRequest, SubscribeEventsResult,
};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Method, Reply};

const SUBSCRIPTION_LIMIT: u16 = 32;
const RETRY: Duration = Duration::from_secs(1);
const HUB_REFRESH: Duration = Duration::from_secs(30);
const HUB_PAGES: usize = 4;

/// One wake per non-empty or lost subscription reply.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlatformEventWake {
    pub lost: bool,
    pub layout_changed: bool,
}

/// Long-poll platform events from the entry Hub and every peer it advertises.
/// A Hub without `SubscribeEvents` ends that origin's watcher; polling fallback
/// still runs for the client as a whole.
pub async fn watch_platform_events(endpoint: Endpoint, wake: mpsc::Sender<PlatformEventWake>) {
    let mut jobs = JoinSet::new();
    let mut watched = BTreeSet::new();
    let mut refresh = tokio::time::interval(HUB_REFRESH);
    refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        if wake.is_closed() {
            break;
        }
        for hub in discover(&endpoint).await {
            if watched.insert(hub) {
                let endpoint = endpoint.clone();
                let wake = wake.clone();
                jobs.spawn(async move { watch_one(endpoint, hub, wake).await });
            }
        }
        tokio::select! {
            _ = refresh.tick() => {}
            _ = wake.closed() => break,
            _ = jobs.join_next(), if !jobs.is_empty() => {}
        }
    }
    jobs.abort_all();
}

async fn watch_one(endpoint: Endpoint, hub: HubId, wake: mpsc::Sender<PlatformEventWake>) {
    let mut after = None;
    let mut primed = false;
    while !wake.is_closed() {
        let Some(mut client) = connect(&endpoint, hub).await else {
            if backoff(&wake).await {
                return;
            }
            continue;
        };
        if !client.hello().methods.contains(&Method::SubscribeEvents) {
            return;
        }
        while !wake.is_closed() {
            let Some(subscription) = subscribe(&mut client, hub, after).await else {
                break;
            };
            if !primed {
                // The caller reconciles with its own snapshot at startup; the
                // retained backlog is not a reason to refresh again.
                primed = true;
                after = Some(head(&subscription));
                continue;
            }
            let lost = subscription.lost;
            let layout_changed = subscription
                .events
                .iter()
                .any(|event| matches!(event.kind, PlatformEventKind::LayoutChanged { .. }));
            after = advance(after, &subscription);
            if lost || !subscription.events.is_empty() {
                if wake
                    .send(PlatformEventWake {
                        lost,
                        layout_changed,
                    })
                    .await
                    .is_err()
                {
                    return;
                }
            }
        }
        if backoff(&wake).await {
            return;
        }
    }
}

fn head(subscription: &SubscribeEventsResult) -> PlatformEventCursor {
    PlatformEventCursor {
        epoch: subscription.stream.epoch,
        sequence: subscription.stream.sequence,
    }
}

/// A lost cursor jumps to the stream head: the retained backlog is skipped
/// because the caller reconciles with a snapshot instead.
pub(crate) fn advance(
    after: Option<PlatformEventCursor>,
    subscription: &SubscribeEventsResult,
) -> Option<PlatformEventCursor> {
    if subscription.lost {
        return Some(head(subscription));
    }
    subscription.next.or(after)
}

async fn connect(endpoint: &Endpoint, hub: HubId) -> Option<EndpointClient> {
    let deadline = Instant::now() + RPC_TIMEOUT;
    endpoint.entry.connect(hub, deadline).await.ok()
}

async fn discover(endpoint: &Endpoint) -> Vec<HubId> {
    let mut hubs = BTreeSet::from([endpoint.hub]);
    let Some(mut client) = connect(endpoint, endpoint.hub).await else {
        return hubs.into_iter().collect();
    };
    let mut after = None;
    for _ in 0..HUB_PAGES {
        let deadline = Instant::now() + RPC_TIMEOUT;
        let Ok(Reply::Hubs { hubs: page, next }) =
            client.call(endpoint.hub, Command::ListHubs { after }, deadline).await
        else {
            break;
        };
        hubs.extend(page.into_iter().map(|hub| hub.hub_id));
        let Some(next) = next else { break };
        after = Some(next);
    }
    hubs.into_iter().collect()
}

async fn subscribe(
    client: &mut EndpointClient,
    hub: HubId,
    after: Option<PlatformEventCursor>,
) -> Option<SubscribeEventsResult> {
    let request = SubscribeEventsRequest::new(SubscribeEventsFields {
        schema_version: PLATFORM_EVENT_SCHEMA_VERSION,
        after,
        limit: SUBSCRIPTION_LIMIT,
        wait_ms: MAX_PLATFORM_EVENT_WAIT_MS,
    })
    .ok()?;
    let deadline =
        Instant::now() + RPC_TIMEOUT + Duration::from_millis(u64::from(MAX_PLATFORM_EVENT_WAIT_MS));
    let reply = client
        .call(hub, Command::SubscribeEvents { request }, deadline)
        .await
        .ok()?;
    match reply {
        Reply::EventSubscription { subscription } => Some(subscription),
        _ => None,
    }
}

async fn backoff(wake: &mpsc::Sender<PlatformEventWake>) -> bool {
    tokio::select! {
        _ = sleep(RETRY) => false,
        _ = wake.closed() => true,
    }
}
