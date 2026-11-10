use std::time::Duration;
use tokio::net::UnixListener;
use yolo_hub_identity::HubId;
use yolo_runtime_protocol::{CatalogueChange, ListSessionsResult, WatchCatalogueStart};
use yolo_terminal_model::HostStatus;
use yolo_runtime_rpc_testing::Peer;
use yolo_wire::{EventEnvelope, ResponseEnvelope, method};

const TIMEOUT: Duration = Duration::from_secs(5);

fn hub() -> HubId {
    HubId::from_uuid(uuid::Uuid::new_v4()).unwrap()
}

/// The client authenticates a managed local Host before any method call.
async fn authenticate(peer: &mut Peer, home: &std::path::Path) {
    let request = peer.request().await.unwrap();
    assert_eq!(request.method, method::HOST_STATUS);
    let status = HostStatus {
        protocol_version: yolo_terminal_model::PROTOCOL_VERSION,
        boot_id: "catalogue-events-test".into(),
        host_pid: std::process::id(),
        controller_generation: 1,
        home: home.to_path_buf(),
        capabilities: Vec::new(),
    };
    peer.respond(&ResponseEnvelope::success(&request, &status).unwrap())
        .await
        .unwrap();
}

#[tokio::test]
async fn catalogue_change_invalidates_the_snapshot_and_wakes_publication() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().canonicalize().unwrap();
    let socket = home.join("host.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let backend =
        yolo_hub_local_backend::LocalBackend::new(hub(), &home, &socket).unwrap();
    let (wake, mut woken) = tokio::sync::watch::channel(0u64);
    let (stop, stopping) = tokio::sync::watch::channel(false);
    let task = tokio::spawn(crate::run(backend.clone(), wake, stopping));

    let (stream, _) = listener.accept().await.unwrap();
    let mut peer = Peer::new(stream);
    authenticate(&mut peer, &home).await;
    let request = peer.request().await.unwrap();
    assert_eq!(request.method, method::WATCH_CATALOGUE);
    let start = WatchCatalogueStart { revision: 0 };
    let response = ResponseEnvelope::success(&request, &start).unwrap();
    peer.respond(&response).await.unwrap();
    let change = CatalogueChange {
        revision: 1,
        terminal_id: None,
    };
    let event = EventEnvelope::new(yolo_wire::event::CATALOGUE_CHANGED, None, None, &change)
        .unwrap();
    peer.event(&event).await.unwrap();
    tokio::time::timeout(TIMEOUT, woken.changed())
        .await
        .unwrap()
        .unwrap();

    // The snapshot was dropped: the next read opens a new Host connection.
    let read = backend.terminals(false);
    let (result, ()) = tokio::join!(read, async {
        let (stream, _) = listener.accept().await.unwrap();
        let mut reader = Peer::new(stream);
        authenticate(&mut reader, &home).await;
        let request = reader.request().await.unwrap();
        assert_eq!(request.method, method::LIST_SESSIONS);
        let listing = ListSessionsResult {
            sessions: Vec::new(),
            complete: true,
        };
        reader
            .respond(&ResponseEnvelope::success(&request, &listing).unwrap())
            .await
            .unwrap();
    });
    assert!(result.unwrap().is_empty());
    stop.send_replace(true);
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn a_revision_gap_resynchronises_once() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().canonicalize().unwrap();
    let socket = home.join("host.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let backend =
        yolo_hub_local_backend::LocalBackend::new(hub(), &home, &socket).unwrap();
    let (wake, mut woken) = tokio::sync::watch::channel(0u64);
    let (stop, stopping) = tokio::sync::watch::channel(false);
    let task = tokio::spawn(crate::run(backend, wake, stopping));

    let (stream, _) = listener.accept().await.unwrap();
    let mut peer = Peer::new(stream);
    authenticate(&mut peer, &home).await;
    let request = peer.request().await.unwrap();
    assert_eq!(request.method, method::WATCH_CATALOGUE);
    let start = WatchCatalogueStart { revision: 9 };
    peer.respond(&ResponseEnvelope::success(&request, &start).unwrap())
        .await
        .unwrap();
    tokio::time::timeout(TIMEOUT, woken.changed())
        .await
        .unwrap()
        .unwrap();
    stop.send_replace(true);
    task.await.unwrap().unwrap();
}
