use super::{ErrorBody, RequestEnvelope, ResponseEnvelope, UnixStream, WorkerState, lock_recover};
use anyhow::Result;
use serde::Serialize;
use std::sync::Weak;
use std::time::Duration;
use tokio::time::Instant;

const RESPONSE_DRAIN_REJECTED_DETAIL: &str =
    "worker shutdown has already been committed; request was not admitted";
const RESPONSE_DRAIN_OVERFLOW_DETAIL: &str = "worker response publication count overflowed";
const RESPONSE_DRAIN_UNDERFLOW_DETAIL: &str =
    "worker response publication resolved without a reservation";
const RESPONSE_DRAIN_TIMEOUT_DETAIL: &str =
    "worker response publication drain exceeded its shutdown deadline";

#[derive(Default)]
pub(super) struct ResponsePublicationState {
    pending: usize,
    shutdown_committed: bool,
    drain_completed: bool,
}

pub(super) struct ResponsePublicationTicket {
    state: Option<Weak<WorkerState>>,
}

impl ResponsePublicationState {
    pub(super) fn reserve(
        state: &std::sync::Arc<WorkerState>,
    ) -> std::result::Result<ResponsePublicationTicket, &'static str> {
        let mut guard = lock_recover(&state.response_publication);
        if guard.shutdown_committed {
            return Err(RESPONSE_DRAIN_REJECTED_DETAIL);
        }
        guard.pending = guard
            .pending
            .checked_add(1)
            .expect(RESPONSE_DRAIN_OVERFLOW_DETAIL);
        Ok(ResponsePublicationTicket {
            state: Some(std::sync::Arc::downgrade(state)),
        })
    }

    pub(super) fn commit_shutdown(&mut self) -> (bool, bool) {
        if self.shutdown_committed {
            return (false, false);
        }
        self.shutdown_committed = true;
        (true, self.complete_drain_if_ready())
    }

    fn resolve(&mut self) -> bool {
        self.pending = self
            .pending
            .checked_sub(1)
            .expect(RESPONSE_DRAIN_UNDERFLOW_DETAIL);
        self.complete_drain_if_ready()
    }

    fn complete_drain_if_ready(&mut self) -> bool {
        if !self.shutdown_committed || self.drain_completed || self.pending != 0 {
            return false;
        }
        self.drain_completed = true;
        true
    }
}

impl ResponsePublicationTicket {
    pub(super) fn publish(mut self) {
        self.resolve();
    }

    #[cfg(test)]
    pub(super) async fn pause_before_write(&self) {
        let Some(state) = self.state.as_ref().and_then(Weak::upgrade) else {
            return;
        };
        state
            .control_test_hooks
            .response_before_write
            .pause_if_armed()
            .await;
    }

    fn resolve(&mut self) {
        let Some(state) = self.state.take().and_then(|weak| weak.upgrade()) else {
            return;
        };
        let should_notify = lock_recover(&state.response_publication).resolve();
        if should_notify {
            state.response_drain_completed.notify_waiters();
        }
    }
}

impl Drop for ResponsePublicationTicket {
    fn drop(&mut self) {
        self.resolve();
    }
}

impl WorkerState {
    pub(super) fn reserve_response_publication(
        self: &std::sync::Arc<Self>,
    ) -> std::result::Result<ResponsePublicationTicket, &'static str> {
        ResponsePublicationState::reserve(self)
    }

    pub(super) fn commit_response_shutdown(&self) {
        let (committed, completed) = lock_recover(&self.response_publication).commit_shutdown();
        if committed {
            self.shutdown.notify_one();
        }
        if completed {
            self.response_drain_completed.notify_waiters();
        }
    }

    pub(super) async fn wait_for_response_drain(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            let notified = self.response_drain_completed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if lock_recover(&self.response_publication).is_released() {
                return true;
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                tracing::warn!("{RESPONSE_DRAIN_TIMEOUT_DETAIL}");
                return false;
            };
            if tokio::time::timeout(remaining, &mut notified)
                .await
                .is_err()
            {
                tracing::warn!("{RESPONSE_DRAIN_TIMEOUT_DETAIL}");
                return false;
            }
        }
    }

    pub(super) fn response_shutdown_is_committed(&self) -> bool {
        lock_recover(&self.response_publication).shutdown_committed
    }
}

pub(super) async fn admit(
    stream: &mut UnixStream,
    state: &std::sync::Arc<WorkerState>,
    request: &RequestEnvelope,
) -> Result<Option<ResponsePublicationTicket>> {
    #[cfg(test)]
    state
        .control_test_hooks
        .response_before_admission
        .pause_if_armed()
        .await;
    match state.reserve_response_publication() {
        Ok(ticket) => Ok(Some(ticket)),
        Err(detail) => {
            let rejection =
                ResponseEnvelope::failure(request, ErrorBody::worker_shutting_down(detail));
            let _ = super::write_worker_frame(stream, &rejection).await;
            Ok(None)
        }
    }
}

pub(super) async fn write<T: Serialize>(
    stream: &mut UnixStream,
    _state: &WorkerState,
    publication: ResponsePublicationTicket,
    response: &T,
) -> Result<()> {
    #[cfg(test)]
    _state
        .control_test_hooks
        .response_before_write
        .pause_if_armed()
        .await;
    super::write_worker_frame(stream, response).await?;
    publication.publish();
    Ok(())
}

#[cfg(test)]
pub(super) async fn pause_before_write(publication: Option<&ResponsePublicationTicket>) {
    if let Some(publication) = publication {
        publication.pause_before_write().await;
    }
}

impl ResponsePublicationState {
    fn is_released(&self) -> bool {
        self.drain_completed
    }
}
