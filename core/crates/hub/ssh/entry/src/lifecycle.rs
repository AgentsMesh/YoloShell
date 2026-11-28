use crate::socket::SocketState;
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::Notify;

#[derive(Default)]
pub(crate) struct Lifecycle {
    closed: AtomicBool,
    changed: Notify,
    socket: Mutex<Option<Weak<SocketState>>>,
}
impl Lifecycle {
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
    pub fn close(&self) {
        self.closed.store(true, Ordering::Release);
        if let Some(socket) = self
            .socket
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
            .and_then(|socket| socket.upgrade())
        {
            socket.close();
        }
        self.changed.notify_waiters();
    }
    pub fn retain(&self, socket: &Arc<SocketState>) -> bool {
        let mut retained = self
            .socket
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if self.is_closed() {
            socket.close();
            return false;
        }
        *retained = Some(Arc::downgrade(socket));
        true
    }
    pub async fn closed(&self) {
        loop {
            let changed = self.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.is_closed() {
                return;
            }
            changed.await;
        }
    }
}
