use crate::constants::{ROOT, SHUTDOWN_TIMEOUT, SOCKET};
use std::{
    future::{Future, poll_fn},
    pin::Pin,
    task::Poll,
};
use tokio::sync::{mpsc, watch};
use uuid::Uuid;
use yolo_hub_client_channels::{CatalogEvent, Endpoint, run};
use yolo_hub_identity::HubId;

const PENDING_CATALOG_CAPACITY: usize = 1;

/// An unavailable private Hub and a deliberately stalled UI. Drive the actual
/// channel owners explicitly so no scheduling delay is used to create pressure.
pub struct CatalogDriver {
    pub refresh: Option<watch::Sender<Option<Uuid>>>,
    pub events: mpsc::Receiver<CatalogEvent>,
    owner: Pin<Box<dyn Future<Output = ()>>>,
    _root: tempfile::TempDir,
}

impl CatalogDriver {
    pub fn new(initial: Uuid) -> Self {
        let root = tempfile::tempdir_in(ROOT).unwrap();
        let endpoint = Endpoint::local(root.path().join(SOCKET), HubId::new());
        let (commands, requests) = mpsc::channel(PENDING_CATALOG_CAPACITY);
        let (maintenance, renewals) = mpsc::channel(PENDING_CATALOG_CAPACITY);
        let (refresh, refreshes) = watch::channel(Some(initial));
        let (controls, _controls) = mpsc::channel(PENDING_CATALOG_CAPACITY);
        let (updates, events) = mpsc::channel(PENDING_CATALOG_CAPACITY);
        drop(commands);
        drop(maintenance);
        let owner = Box::pin(run(
            endpoint, requests, renewals, refreshes, controls, updates,
        ));
        Self {
            refresh: Some(refresh),
            events,
            owner,
            _root: root,
        }
    }

    pub async fn advance_to_pending(&mut self) {
        poll_fn(|cx| {
            assert!(
                self.owner.as_mut().poll(cx).is_pending(),
                "catalog owner exited early"
            );
            Poll::Ready(())
        })
        .await;
    }

    pub async fn blocked(&mut self) -> Uuid {
        self.advance_to_pending().await;
        assert_eq!(self.events.len(), PENDING_CATALOG_CAPACITY);
        let blocked = self.request();
        self.advance_to_pending().await;
        assert_eq!(self.events.len(), PENDING_CATALOG_CAPACITY);
        blocked
    }

    pub fn request(&self) -> Uuid {
        let id = Uuid::new_v4();
        self.refresh.as_ref().unwrap().send_replace(Some(id));
        id
    }

    pub fn failed(&mut self, expected: Uuid) {
        let event = self.events.try_recv().unwrap();
        assert_eq!(event.request_id, expected);
        assert!(
            event.result.is_err(),
            "the private Hub socket does not exist"
        );
    }

    pub async fn complete(&mut self) {
        tokio::time::timeout(SHUTDOWN_TIMEOUT, &mut self.owner)
            .await
            .unwrap();
    }
}
