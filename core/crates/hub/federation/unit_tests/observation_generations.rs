use super::*;
#[path = "../test_support/node_inventory.rs"]
mod fixture;
use fixture::NodeFixture;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};
use yolo_hub_federation_settings::routing_limits;

const SHORT_WAIT: Duration = Duration::from_millis(20);
const REPLACEMENTS: usize = 3;

#[tokio::test]
async fn timed_out_or_cancelled_workers_bound_successor_startups_until_they_really_exit() {
    for cancel in [false, true] {
        let fixture = NodeFixture::new().await;
        let node = &fixture.node;
        node.observation_cache.retire();
        node.observation_cache
            .wait_closed(Instant::now() + STOP_TIMEOUT)
            .await;
        let state = node.state.clone();
        let hub = state.hub_id();
        let starts = Arc::new(AtomicUsize::new(0));
        let observed_starts = starts.clone();
        let (release, blocked) = mpsc::channel();
        let (entered, started) = oneshot::channel();
        let running = tokio::spawn(async move {
            let mut book =
                OriginBook::new(state.peer_identity(), Vec::new(), routing_limits()).unwrap();
            ObservationCache::start(
                hub,
                move |retirement| {
                    observed_starts.fetch_add(1, Ordering::SeqCst);
                    let _ = entered.send(retirement.clone());
                    let _ = blocked.recv();
                    observation_worker::open(state, retirement)
                },
                &mut book,
                Instant::now() + if cancel { STOP_TIMEOUT } else { SHORT_WAIT },
            )
            .await
        });
        let retired = timeout_at(Instant::now() + STOP_TIMEOUT, started)
            .await
            .unwrap()
            .unwrap();
        let previous = if cancel {
            running.abort();
            assert!(matches!(running.await, Err(error) if error.is_cancelled()));
            None
        } else {
            Some(running.await.unwrap())
        };
        assert!(retired.is_retired());
        let mut book =
            OriginBook::new(node.state.peer_identity(), Vec::new(), routing_limits()).unwrap();
        for _ in 0..REPLACEMENTS {
            let starts = starts.clone();
            let state = node.state.clone();
            let blocked = ObservationCache::start(
                hub,
                move |retirement| {
                    starts.fetch_add(1, Ordering::SeqCst);
                    observation_worker::open(state, retirement)
                },
                &mut book,
                Instant::now() + STOP_TIMEOUT,
            )
            .await;
            assert!(!blocked.mailbox.available.load(Ordering::Acquire));
            assert!(*blocked.closed.borrow());
        }
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        assert!(book.history().catalogues().next().is_none());
        release.send(()).unwrap();
        if let Some(previous) = previous {
            previous.wait_closed(Instant::now() + STOP_TIMEOUT).await;
            assert!(*previous.closed.borrow());
        } else {
            timeout_at(Instant::now() + STOP_TIMEOUT, async {
                loop {
                    if let Some(admission) = Admission::acquire(hub) {
                        drop(admission);
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
        }
        let recovered = ObservationCache::open(node.state.clone(), &mut book).await;
        assert!(recovered.mailbox.available.load(Ordering::Acquire));
        recovered.retire();
        recovered.wait_closed(Instant::now() + STOP_TIMEOUT).await;
    }
}
