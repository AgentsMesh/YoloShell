#[path = "../test_support/network_capacity_constants.rs"]
mod constants;
#[path = "../test_support/network_capacity_fixture.rs"]
mod fixture;
#[path = "../test_support/network_capacity_observation.rs"]
mod observation;
#[path = "../test_support/network_capacity_peer.rs"]
mod peer;
#[path = "../test_support/network_capacity_pool.rs"]
mod pool;
#[path = "../test_support/network_capacity_running.rs"]
mod running;

use crate::discovery_channel::DiscoveryRefresh;
use constants::*;
use fixture::LinkFixture;
use pool::Pressure;
use running::Running;
use std::sync::atomic::Ordering;
use tokio::{
    sync::oneshot::error::TryRecvError,
    time::{sleep, timeout},
};

#[tokio::test]
async fn local_capacity_defers_discovery_without_retiring_link_but_bad_signatures_retire() {
    let mut fixture = LinkFixture::new().await;
    let pressure = Pressure::rpc(&fixture).await;
    assert_eq!(
        fixture.node.discover(&fixture.link).await.unwrap(),
        DiscoveryRefresh::DeferredLocalAdmission,
    );
    let mut running = Running::start(&mut fixture);
    sleep(crate::POLL_INTERVAL * OBSERVED_POLL_CYCLES).await;
    assert_eq!(
        running.finished.try_recv().unwrap_err(),
        TryRecvError::Empty
    );
    fixture.assert_retained().await;

    drop(pressure);
    fixture.wait_refreshed().await;
    fixture.assert_retained().await;

    fixture.responder.invalid.store(true, Ordering::Release);
    timeout(TEST_TIMEOUT, &mut running.finished)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(
        fixture
            .node
            .current(fixture.link.peer.hub())
            .await
            .is_none()
    );
    assert!(fixture.link.is_closed());
    assert!(fixture.node.discover(&fixture.link).await.is_err());
    running.close().await;
    fixture.close().await;
}
