use crate::{
    constants::*, observation_admission::Admission, observation_mailbox::Mailbox,
    observation_worker,
};
use std::sync::Arc;
use tokio::{
    sync::{oneshot, watch},
    time::{Instant, timeout_at},
};
use yolo_hub_federation_directory::{ObservationHistory, OriginBook};
use yolo_hub_identity::HubId;
use yolo_hub_observation_store::{ObservationStore, Retirement};
use yolo_hub_state::HubState;

/// One cache worker owns storage; callers only replace its bounded pending snapshot.
pub(crate) struct ObservationCache {
    pub(crate) mailbox: Arc<Mailbox>,
    closed: watch::Receiver<bool>,
}

impl ObservationCache {
    pub async fn open(state: Arc<HubState>, book: &mut OriginBook) -> Self {
        Self::start(
            state.hub_id(),
            move |retirement| observation_worker::open(state, retirement),
            book,
            Instant::now() + OBSERVATION_LOAD_TIMEOUT,
        )
        .await
    }

    async fn start(
        hub: HubId,
        open: impl FnOnce(Retirement) -> anyhow::Result<(ObservationStore, ObservationHistory)>
        + Send
        + 'static,
        book: &mut OriginBook,
        deadline: Instant,
    ) -> Self {
        let (ready, loaded) = oneshot::channel();
        let (finished, closed) = watch::channel(false);
        let mailbox = Arc::new(Mailbox::default());
        let cache = Self {
            mailbox: mailbox.clone(),
            closed,
        };
        let Some(admission) = Admission::acquire(hub) else {
            tracing::warn!(%hub, message = OBSERVATION_WORKER_BUSY);
            cache.retire();
            finished.send_replace(true);
            return cache;
        };
        let worker = std::thread::Builder::new()
            .name(OBSERVATION_WORKER.into())
            .spawn(move || observation_worker::run(open, mailbox, ready, finished, admission));
        if let Err(error) = worker {
            tracing::warn!(%error, message = OBSERVATION_CACHE_UNAVAILABLE);
            cache.retire();
            return cache;
        }
        match timeout_at(deadline, loaded).await {
            Ok(Ok(history)) => {
                if let Err(error) = book.restore_history(history) {
                    tracing::warn!(%error, message = OBSERVATION_CACHE_UNAVAILABLE);
                    cache.retire();
                }
            }
            result => {
                tracing::warn!(?result, message = OBSERVATION_CACHE_UNAVAILABLE);
                cache.retire();
            }
        }
        cache
    }

    pub fn record(&self, history: ObservationHistory) -> Option<oneshot::Receiver<()>> {
        self.mailbox.record(history)
    }

    pub fn retire(&self) {
        self.mailbox.retire();
    }

    pub async fn wait_closed(&self, deadline: Instant) {
        let mut closed = self.closed.clone();
        let stopped = async {
            while !*closed.borrow_and_update() {
                if closed.changed().await.is_err() {
                    break;
                }
            }
        };
        if timeout_at(deadline, stopped).await.is_err() {
            tracing::warn!(message = OBSERVATION_WORKER_STALLED);
        }
    }
}

impl Drop for ObservationCache {
    fn drop(&mut self) {
        self.retire();
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/observation_cache.rs"]
mod unit_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/observation_deadlines.rs"]
mod deadline_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/observation_generations.rs"]
mod generation_tests;
