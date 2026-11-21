use crate::{
    constants::*,
    platform::{Subscription, serve},
};
use tokio::{net::UnixListener, sync::mpsc, time::timeout};
use uuid::Uuid;
use yolo_hub_client_channels::{Endpoint, watch_platform_events};
use yolo_hub_events_protocol::{
    PlatformEvent, PlatformEventCursor, PlatformEventKind, PlatformEventStream,
    SubscribeEventsResult,
};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Method;
use yolo_presentation_protocol::LayoutAddress;

const QUIET: std::time::Duration = std::time::Duration::from_millis(200);

fn stream(sequence: u64) -> PlatformEventStream {
    PlatformEventStream {
        epoch: Uuid::new_v4(),
        sequence,
        first_sequence: 1,
    }
}

fn head(stream: &PlatformEventStream) -> PlatformEventCursor {
    PlatformEventCursor {
        epoch: stream.epoch,
        sequence: stream.sequence,
    }
}

fn result(
    stream: PlatformEventStream,
    lost: bool,
    timed_out: bool,
    events: Vec<PlatformEvent>,
) -> SubscribeEventsResult {
    SubscribeEventsResult {
        stream,
        events,
        next: None,
        lost,
        timed_out,
    }
}

async fn endpoint() -> (tempfile::TempDir, Endpoint, UnixListener) {
    let root = tempfile::tempdir_in(ROOT).unwrap();
    let endpoint = Endpoint::local(root.path().join(SOCKET), HubId::new());
    let listener = UnixListener::bind(endpoint.entry.local_socket().unwrap()).unwrap();
    (root, endpoint, listener)
}

#[tokio::test]
async fn a_hub_without_subscribe_events_ends_the_watcher() {
    let (_root, endpoint, listener) = endpoint().await;
    let (subscriptions, mut calls) = mpsc::channel(CAPACITY);
    let (refusals, mut refused) = mpsc::channel(CAPACITY);
    let server = tokio::spawn(serve(
        listener,
        endpoint.hub,
        vec![Method::Hello, Method::ListHubs],
        subscriptions,
        refusals,
    ));
    let (wake, wakes) = mpsc::channel(CAPACITY);
    let task = tokio::spawn(watch_platform_events(endpoint.clone(), wake));
    timeout(TIMEOUT, refused.recv()).await.unwrap();
    drop(wakes);
    timeout(SHUTDOWN_TIMEOUT, task).await.unwrap().unwrap();
    assert!(calls.try_recv().is_err());
    server.abort();
}

#[tokio::test]
async fn watcher_primes_then_wakes_on_a_lost_cursor_and_a_layout_change() {
    let (_root, endpoint, listener) = endpoint().await;
    let (subscriptions, mut calls) = mpsc::channel(CAPACITY);
    let (refusals, _refused) = mpsc::channel(CAPACITY);
    let server = tokio::spawn(serve(
        listener,
        endpoint.hub,
        vec![Method::Hello, Method::ListHubs, Method::SubscribeEvents],
        subscriptions,
        refusals,
    ));
    let (wake, mut wakes) = mpsc::channel(CAPACITY);
    let task = tokio::spawn(watch_platform_events(endpoint.clone(), wake));

    // The first reply primes the cursor at the stream head; a retained
    // backlog is not replayed and does not wake the caller.
    let prime: Subscription = calls.recv().await.unwrap();
    assert_eq!(prime.after, None);
    let boot = stream(5);
    prime
        .respond
        .send(result(boot.clone(), false, false, Vec::new()))
        .unwrap();
    assert!(timeout(QUIET, wakes.recv()).await.is_err());

    let second = calls.recv().await.unwrap();
    assert_eq!(second.after, Some(head(&boot)));
    let later = stream(9);
    second
        .respond
        .send(result(later.clone(), true, false, Vec::new()))
        .unwrap();
    let notice = timeout(TIMEOUT, wakes.recv()).await.unwrap().unwrap();
    assert!(notice.lost);
    assert!(!notice.layout_changed);

    // A lost cursor continues from the new stream head.
    let jump = calls.recv().await.unwrap();
    assert_eq!(jump.after, Some(head(&later)));
    jump.respond
        .send(result(stream(0), false, true, Vec::new()))
        .unwrap();
    assert!(timeout(QUIET, wakes.recv()).await.is_err());

    // A layout change is a wake reason of its own, without a lost cursor.
    let layout = calls.recv().await.unwrap();
    let event = PlatformEvent {
        sequence: 10,
        at_ms: 0,
        kind: PlatformEventKind::LayoutChanged {
            address: LayoutAddress {
                hub_id: endpoint.hub,
                layout_id: Uuid::new_v4(),
            },
            revision: 1,
        },
    };
    layout
        .respond
        .send(result(stream(10), false, false, vec![event]))
        .unwrap();
    let notice = timeout(TIMEOUT, wakes.recv()).await.unwrap().unwrap();
    assert!(!notice.lost);
    assert!(notice.layout_changed);

    drop(wakes);
    timeout(SHUTDOWN_TIMEOUT, task).await.unwrap().unwrap();
    server.abort();
}

