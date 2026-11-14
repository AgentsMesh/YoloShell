use std::sync::Arc;
use tokio::sync::{Mutex, Notify, oneshot, watch};
use yolo_capnp_rpc::Error;

/// Authenticated setup stays cancellable until the broker promotes this exact connection.
pub(super) struct RuntimeAdmission {
    ready: Mutex<Option<oneshot::Sender<()>>>,
    allowed: watch::Receiver<bool>,
    seen: Arc<Notify>,
}
impl RuntimeAdmission {
    pub fn new(
        ready: oneshot::Sender<()>,
        allowed: watch::Receiver<bool>,
        seen: Arc<Notify>,
    ) -> Self {
        Self {
            ready: Mutex::new(Some(ready)),
            allowed,
            seen,
        }
    }
    pub fn started(&self) {
        self.seen.notify_one();
    }
    pub async fn wait(&self) -> Result<(), Error> {
        if let Some(ready) = self.ready.lock().await.take() {
            ready.send(()).map_err(|_| Error::Closed)?;
        }
        let mut allowed = self.allowed.clone();
        if !*allowed.borrow() {
            allowed.changed().await.map_err(|_| Error::Closed)?;
        }
        if *allowed.borrow() {
            Ok(())
        } else {
            Err(Error::Closed)
        }
    }
}
