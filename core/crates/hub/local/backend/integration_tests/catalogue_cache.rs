use super::{cleanup_rpc::*, fixtures::*};
use std::time::Duration;
use yolo_hub_rpc_protocol::Command;
use yolo_runtime::{
    host::ListSessionsResult,
    protocol::{ResponseEnvelope, method},
};

fn cached() -> ProtocolFixture {
    const CACHE_TTL: Duration = Duration::from_secs(1);
    let mut fixture = ProtocolFixture::new();
    fixture.backend.catalogue =
        std::sync::Arc::new(yolo_hub_host_catalogue::CatalogueCache::new(CACHE_TTL));
    fixture
}

fn listing(
    sessions: Vec<yolo_terminal_model::TerminalView>,
) -> impl FnOnce(&yolo_runtime::protocol::RequestEnvelope) -> ResponseEnvelope {
    move |request| {
        assert_eq!(request.method, method::LIST_SESSIONS);
        ResponseEnvelope::success(
            request,
            &ListSessionsResult {
                sessions,
                complete: true,
            },
        )
        .unwrap()
    }
}

/// One client command reads the catalogue several times; a single Host read
/// must serve all of them.
#[tokio::test]
async fn repeated_listings_share_one_host_catalogue_read() {
    let fixture = cached();
    let (first, ()) = bounded(async {
        tokio::join!(fixture.backend.terminals(false), fixture.reply(listing(Vec::new())))
    })
    .await;
    assert!(first.unwrap().is_empty());
    let second = fixture.backend.terminals(false).await.unwrap();
    assert!(second.is_empty());
    fixture.assert_no_pending_connection();
}

/// A catalogue mutation drops the snapshot, so the next read sees the Host.
#[tokio::test]
async fn catalogue_mutation_invalidates_the_snapshot() {
    let fixture = cached();
    let (first, ()) = bounded(async {
        tokio::join!(
            fixture.backend.terminals(false),
            fixture.reply(listing(vec![view()]))
        )
    })
    .await;
    assert_eq!(first.unwrap().len(), 1);
    let command = Command::CleanupArchive { creation: creation() };
    let (result, ()) = bounded(async {
        tokio::join!(
            fixture.backend.execute(&command),
            fixture.reply(|request| {
                assert_eq!(request.method, method::CLEANUP_ARCHIVE_SESSION);
                let mut archived = view();
                archived.manifest.archived = true;
                ResponseEnvelope::success(request, &archived).unwrap()
            })
        )
    })
    .await;
    assert!(result.is_ok());
    let (refetched, ()) = bounded(async {
        tokio::join!(
            fixture.backend.terminals(false),
            fixture.reply(listing(Vec::new()))
        )
    })
    .await;
    assert!(refetched.unwrap().is_empty());
    fixture.assert_no_pending_connection();
}

/// A complete snapshot also serves archived-free reads, so alternating list
/// and list-agents calls within one command do not refetch.
#[tokio::test]
async fn complete_snapshot_serves_archived_free_reads() {
    let fixture = cached();
    let live = view();
    let mut archived = view();
    archived.manifest.terminal_id = uuid::Uuid::new_v4();
    archived.manifest.archived = true;
    let (all, ()) = bounded(async {
        tokio::join!(
            fixture.backend.terminals(true),
            fixture.reply(listing(vec![live.clone(), archived.clone()]))
        )
    })
    .await;
    assert_eq!(all.unwrap().len(), 2);
    let visible = fixture.backend.terminals(false).await.unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(
        visible[0].fields().address.terminal_id.as_uuid(),
        live.manifest.terminal_id
    );
    fixture.assert_no_pending_connection();
}
