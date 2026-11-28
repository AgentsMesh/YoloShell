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
