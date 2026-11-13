use super::*;
use crate::protocol::{EventEnvelope, RequestEnvelope, ResponseEnvelope, event, method, to_value};
use tempfile::tempdir;
use yolo_runtime_protocol::{CatalogueChange, WatchCatalogueRequest, WatchCatalogueStart};

const CATALOGUE_WATCH_TEST_TIMEOUT: Duration = Duration::from_secs(5);
const CATALOGUE_WATCH_REQUEST_ID: &str = "catalogue-watch-request";

/// A Hub subscriber receives revision-stamped change hints from one stream,
/// and the stream carries no catalogue views or terminal output.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn catalogue_watch_streams_revision_stamped_change_hints() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let (client, server) = UnixStream::pair().unwrap();
    let client_host = Arc::clone(&host);
    tokio::spawn(async move {
        let _ = client_host.handle_client(server).await;
    });
    let connection = yolo_capnp_rpc::Connection::open(
        client,
        yolo_capnp_rpc::Protocol::Endpoint,
        tokio::time::Instant::now() + CATALOGUE_WATCH_TEST_TIMEOUT,
    )
    .await
    .unwrap();
    let request = RequestEnvelope::new(
        CATALOGUE_WATCH_REQUEST_ID,
        method::WATCH_CATALOGUE,
        to_value(&WatchCatalogueRequest { after_revision: 0 }).unwrap(),
    );
    let reply = connection
        .request(
            yolo_capnp_rpc::Method::Call,
            yolo_capnp_codec::to_value(&request).unwrap(),
            tokio::time::Instant::now() + CATALOGUE_WATCH_TEST_TIMEOUT,
        )
        .await
        .unwrap();
    let response: ResponseEnvelope = crate::protocol::from_value(reply.value).unwrap();
    let start: WatchCatalogueStart = response.decode_payload().unwrap();
    assert_eq!(start.revision, 0);
    let mut events = reply.events.expect("catalogue watch stream");
    let terminal_id = Uuid::new_v4();
    host.watch.feed.touch(Some(terminal_id));
    let value = events
        .next(tokio::time::Instant::now() + CATALOGUE_WATCH_TEST_TIMEOUT)
        .await
        .unwrap();
    let envelope: EventEnvelope = crate::protocol::from_value(value).unwrap();
    assert_eq!(envelope.event, event::CATALOGUE_CHANGED);
    let change: CatalogueChange = crate::protocol::from_value(envelope.payload).unwrap();
    assert_eq!(change.revision, 1);
    assert_eq!(change.terminal_id, Some(terminal_id));
}
