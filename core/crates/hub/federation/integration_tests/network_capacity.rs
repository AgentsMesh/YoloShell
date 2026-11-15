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
    io::AsyncReadExt,
    sync::oneshot::error::TryRecvError,
    time::{sleep, timeout},
};

enum Pool {
    Ssh,
    Rpc,
}

#[tokio::test]
async fn local_capacity_defers_discovery_without_retiring_streams_but_bad_signatures_retire() {
    // Both cases share the actual process-global RPC engine; run them serially.
    for pool in [Pool::Ssh, Pool::Rpc] {
        let mut fixture = LinkFixture::new().await;
        let pressure = match pool {
            Pool::Ssh => Pressure::ssh(&mut fixture).await,
            Pool::Rpc => Pressure::rpc().await,
        };
        assert!(pressure.retained() > usize::default());
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
                .current(fixture.link.connection.peer().hub())
                .await
                .is_none()
        );
        assert!(fixture.link.connection.is_closed());
        assert!(
            timeout(TEST_TIMEOUT, fixture.output.read_u8())
                .await
                .unwrap()
                .is_err()
        );
        assert!(fixture.node.discover(&fixture.link).await.is_err());
        running.close().await;
        fixture.close().await;
    }
}
