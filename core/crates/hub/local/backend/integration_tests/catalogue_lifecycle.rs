use super::{cleanup_rpc::*, fixtures::view, native_support::live_listing};
use std::sync::Arc;
use std::time::Duration;
use yolo_terminal_model::TerminalRuntimeStatus;

const TTL: Duration = Duration::from_millis(5);

#[tokio::test]
async fn only_settled_catalogues_with_a_live_subscription_outlast_the_ttl() {
    let mut fixture = ProtocolFixture::new();
    fixture.backend.catalogue = Arc::new(yolo_hub_host_catalogue::CatalogueCache::new(TTL));
    let watch = fixture.backend.follow_catalogue_lifecycle().await;
    let mut ended = view();
    ended.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Lost;
    let fetch = |view| async {
        let views = [view];
        let (result, ()) = bounded(async {
            tokio::join!(
                fixture.backend.terminals(false),
                fixture.reply(|r| live_listing(r, &views))
            )
        })
        .await;
        assert_eq!(result.unwrap().len(), 1);
    };
    fetch(ended.clone()).await;
    tokio::time::sleep(TTL).await;
    assert_eq!(
        bounded(fixture.backend.terminals(false))
            .await
            .unwrap()
            .len(),
        1
    );
    fixture.assert_no_pending_connection();
    fixture.backend.invalidate_catalogue().await;
    fetch(view()).await;
    tokio::time::sleep(TTL).await;
    fetch(ended).await;
    drop(watch);
    tokio::time::sleep(TTL).await;
    fetch(view()).await;
    fixture.assert_no_pending_connection();
}

#[tokio::test]
async fn a_live_row_does_not_force_worker_and_history_reads_for_its_settled_neighbors() {
    use super::native_support::session;
    let mut f = ProtocolFixture::new();
    f.backend.catalogue = Arc::new(yolo_hub_host_catalogue::CatalogueCache::new(TTL));
    let _watch = f.backend.follow_catalogue_lifecycle().await;
    let active = view();
    let mut old = view();
    old.manifest.terminal_id = uuid::Uuid::new_v4();
    old.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Lost;
    let baseline = [active.clone(), old.clone()];
    let state = |rows: &[yolo_terminal_model::TerminalView]| {
        rows.iter()
            .map(|row| {
                (
                    row.manifest.terminal_id,
                    row.runtime.as_ref().unwrap().status,
                )
            })
            .collect::<Vec<_>>()
    };
    let (initial, ()) = bounded(async {
        tokio::join!(
            f.backend.catalogue_views(false),
            f.reply(|r| live_listing(r, &baseline))
        )
    })
    .await;
    assert_eq!(state(&initial.unwrap()), state(&baseline));

    tokio::time::sleep(TTL).await;
    let mut finished = active.clone();
    finished.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Exited;
    let (refresh, ()) = bounded(async {
        tokio::join!(
            f.backend.catalogue_views(false),
            f.reply(|r| session(r, &finished))
        )
    })
    .await;
    assert_eq!(
        state(&refresh.unwrap()),
        state(&[finished.clone(), old.clone()])
    );

    f.backend
        .catalogue_changed(&[old.manifest.terminal_id].into())
        .await;
    old.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Running;
    let (reopened, ()) = bounded(async {
        tokio::join!(
            f.backend.catalogue_views(false),
            f.reply(|r| session(r, &old))
        )
    })
    .await;
    assert_eq!(
        state(&reopened.unwrap()),
        state(&[finished.clone(), old.clone()])
    );

    let mut new = view();
    new.manifest.terminal_id = uuid::Uuid::new_v4();
    f.backend
        .catalogue_changed(&[new.manifest.terminal_id].into())
        .await;
    let changed = [finished, old, new];
    let (resync, ()) = bounded(async {
        tokio::join!(
            f.backend.catalogue_views(false),
            f.reply(|r| live_listing(r, &changed))
        )
    })
    .await;
    assert_eq!(state(&resync.unwrap()), state(&changed));
    f.assert_no_pending_connection();
}
