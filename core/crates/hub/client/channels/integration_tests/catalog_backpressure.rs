use crate::catalog_driver::CatalogDriver;
use uuid::Uuid;

#[tokio::test]
async fn replacement_during_ui_backpressure_discards_superseded_catalog_result() {
    let first = Uuid::new_v4();
    let mut driver = CatalogDriver::new(first);
    let blocked = driver.blocked().await;
    let latest = driver.request();
    assert_ne!(blocked, latest);
    driver.advance_to_pending().await;
    driver.failed(first);
    driver.advance_to_pending().await;
    driver.failed(latest);
    assert!(driver.events.try_recv().is_err());
    driver.refresh.take();
    driver.complete().await;
    assert!(driver.events.recv().await.is_none());
}

#[tokio::test]
async fn closing_refresh_while_ui_is_full_cancels_pending_catalog_publication() {
    let first = Uuid::new_v4();
    let mut driver = CatalogDriver::new(first);
    driver.blocked().await;
    driver.refresh.take();
    driver.complete().await;
    driver.failed(first);
    assert!(driver.events.recv().await.is_none());
}

#[tokio::test]
async fn closing_ui_while_publication_is_blocked_releases_catalog_owner() {
    let first = Uuid::new_v4();
    let mut driver = CatalogDriver::new(first);
    driver.blocked().await;
    driver.events.close();
    driver.complete().await;
    driver.failed(first);
    assert!(driver.events.recv().await.is_none());
}

#[tokio::test]
async fn catalog_shutdown_before_start_never_publishes_initial_request() {
    for producer_closed in [false, true] {
        let mut driver = CatalogDriver::new(Uuid::new_v4());
        if producer_closed {
            driver.refresh.take();
        } else {
            driver.events.close();
        }
        driver.complete().await;
        assert!(driver.events.recv().await.is_none());
    }
}
