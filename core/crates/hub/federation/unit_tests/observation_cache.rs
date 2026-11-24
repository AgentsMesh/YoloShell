use super::*;
#[path = "../test_support/node_inventory.rs"]
mod fixture;
use fixture::NodeFixture;
use std::sync::atomic::Ordering;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_observation_store::OBSERVATION_DIRECTORY;
use yolo_hub_peer_protocol_test_support::support::observe;

const SAVED_CACHE: &str = "saved-observations";
const CACHE_OBSTRUCTION: &[u8] = b"unrelated file retained by cache recovery";

#[tokio::test]
async fn persisted_observations_restore_without_a_connection_export_or_control_route() {
    let fixture = NodeFixture::new().await;
    let node = &fixture.node;
    let history = node.history().await;
    let cache = &node.observation_cache;
    timeout_at(
        Instant::now() + STOP_TIMEOUT,
        cache.record(history.clone()).unwrap(),
    )
    .await
    .unwrap()
    .unwrap();
    cache.retire();
    cache.wait_closed(Instant::now() + STOP_TIMEOUT).await;
    let mut book = OriginBook::new(
        node.state.peer_identity(),
        node.config.pins(),
        routing_limits(),
    )
    .unwrap();
    let restored = ObservationCache::open(node.state.clone(), &mut book).await;
    assert!(restored.mailbox.available.load(Ordering::Acquire));
    let now = crate::node::now_ms().unwrap();
    assert!(book.latest_for(fixture.origin.identity.hub()).is_some());
    assert!(book.hubs(now).unwrap().iter().all(|hub| !hub.reachable));
    assert!(book.exportable(now).unwrap().is_empty());
    assert!(
        book.route_for(fixture.origin.identity.hub(), &observe(), now)
            .unwrap()
            .is_none()
    );
    assert_eq!(book.history(), history);
    assert!(!fixture.host_socket.exists());
    restored.retire();
    restored.wait_closed(Instant::now() + STOP_TIMEOUT).await;
}

#[tokio::test]
async fn storage_loss_disables_the_cache_without_revoking_authenticated_live_routes() {
    let fixture = NodeFixture::new().await;
    let node = &fixture.node;
    let history = node.history().await;
    let cache = &node.observation_cache;
    timeout_at(
        Instant::now() + STOP_TIMEOUT,
        cache.record(history.clone()).unwrap(),
    )
    .await
    .unwrap()
    .unwrap();
    let root = node.state.directory().unwrap();
    let path = root.path().join(OBSERVATION_DIRECTORY);
    std::fs::rename(&path, root.path().join(SAVED_CACHE)).unwrap();
    std::fs::write(&path, CACHE_OBSTRUCTION).unwrap();
    let failed = cache.record(history.clone()).unwrap();
    assert!(
        timeout_at(Instant::now() + STOP_TIMEOUT, failed)
            .await
            .unwrap()
            .is_err()
    );
    cache.wait_closed(Instant::now() + STOP_TIMEOUT).await;
    assert!(!cache.mailbox.available.load(Ordering::Acquire));
    assert!(cache.record(history.clone()).is_none());
    assert_eq!(std::fs::read(&path).unwrap(), CACHE_OBSTRUCTION);
    node.require_open().unwrap();
    let now = crate::node::now_ms().unwrap();
    let book = node.book.lock().await;
    assert!(
        book.route_for(fixture.origin.identity.hub(), &observe(), now)
            .unwrap()
            .is_some()
    );
    assert!(book.hubs(now).unwrap().iter().any(|hub| hub.reachable));
    assert_eq!(book.history(), history);
    assert!(!fixture.host_socket.exists());
}

#[tokio::test]
async fn unavailable_cache_preserves_conflicting_storage_and_an_empty_local_directory() {
    let fixture = NodeFixture::new().await;
    let node = &fixture.node;
    node.observation_cache.retire();
    node.observation_cache
        .wait_closed(Instant::now() + STOP_TIMEOUT)
        .await;
    let root = node.state.directory().unwrap();
    let path = root.path().join(OBSERVATION_DIRECTORY);
    std::fs::rename(&path, root.path().join(SAVED_CACHE)).unwrap();
    std::fs::write(&path, CACHE_OBSTRUCTION).unwrap();
    let mut book = OriginBook::new(
        node.state.peer_identity(),
        node.config.pins(),
        routing_limits(),
    )
    .unwrap();
    let restored = ObservationCache::open(node.state.clone(), &mut book).await;
    assert!(!restored.mailbox.available.load(Ordering::Acquire));
    assert!(
        book.hubs(crate::node::now_ms().unwrap())
            .unwrap()
            .is_empty()
    );
    assert_eq!(std::fs::read(path).unwrap(), CACHE_OBSTRUCTION);
    node.require_open().unwrap();
    assert!(!fixture.host_socket.exists());
}

#[tokio::test]
async fn run_with_shutdown_already_requested_releases_cache_for_the_next_generation() {
    let fixture = NodeFixture::new().await;
    let node = &fixture.node;
    assert!(
        node.observation_cache
            .mailbox
            .available
            .load(Ordering::Acquire)
    );
    let (_stop, stopped) = watch::channel(true);
    node.clone().run(None, stopped).await.unwrap();
    assert!(*node.observation_cache.closed.borrow());
    let mut book = OriginBook::new(
        node.state.peer_identity(),
        node.config.pins(),
        routing_limits(),
    )
    .unwrap();
    let successor = ObservationCache::open(node.state.clone(), &mut book).await;
    assert!(successor.mailbox.available.load(Ordering::Acquire));
    successor.retire();
    successor.wait_closed(Instant::now() + STOP_TIMEOUT).await;
}
