use super::bundle_raw::CANCELLED;
use crate::remote::RemoteError;
use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::Notify;

#[derive(Clone, Default)]
pub(super) struct Cancellation(Arc<State>);

#[derive(Default)]
struct State {
    cancelled: AtomicBool,
    changed: Notify,
}

impl Cancellation {
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
        self.0.changed.notify_waiters();
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }
    async fn changed(&self) {
        loop {
            let notified = self.0.changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
    pub async fn run<T>(
        &self,
        future: impl Future<Output = Result<T, RemoteError>>,
    ) -> Result<T, RemoteError> {
        tokio::select! { biased;
            _ = self.changed() => Err(RemoteError::RuntimeDeployment(CANCELLED.into())),
            result = future => result,
        }
    }
}
