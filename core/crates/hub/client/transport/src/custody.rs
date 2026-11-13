use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Waker},
};

type CloseOnce = Box<dyn FnOnce() + Send + Sync>;

pub(crate) struct CloseState {
    closed: AtomicBool,
    callback: Mutex<Option<CloseOnce>>,
    readers: Mutex<Option<Waker>>,
    writers: Mutex<Option<Waker>>,
    changed: tokio::sync::Notify,
}

impl CloseState {
    pub fn new(close: impl FnOnce() + Send + Sync + 'static) -> Arc<Self> {
        Arc::new(Self {
            closed: AtomicBool::new(false),
            callback: Mutex::new(Some(Box::new(close))),
            readers: Mutex::new(None),
            writers: Mutex::new(None),
            changed: tokio::sync::Notify::new(),
        })
    }
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
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
    pub fn reading(&self, context: &Context<'_>) {
        remember(&self.readers, context.waker());
    }
    pub fn writing(&self, context: &Context<'_>) {
        remember(&self.writers, context.waker());
    }
    pub fn close(&self) {
        if self.closed.swap(true, Ordering::AcqRel) {
            return;
        }
        self.changed.notify_waiters();
        for waiter in [&self.readers, &self.writers] {
            let wake = waiter
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .take();
            if let Some(wake) = wake {
                wake.wake();
            }
        }
        let callback = self
            .callback
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some(close) = callback {
            close();
        }
    }
}

fn remember(slot: &Mutex<Option<Waker>>, next: &Waker) {
    let mut slot = slot.lock().unwrap_or_else(|error| error.into_inner());
    if slot
        .as_ref()
        .is_none_or(|previous| !previous.will_wake(next))
    {
        *slot = Some(next.clone());
    }
}

pub(crate) struct IoCustody(pub Arc<CloseState>);
impl Drop for IoCustody {
    fn drop(&mut self) {
        self.0.close();
    }
}

/// An interrupted channel open has an unknown SSH result: retire its connection.
pub(crate) struct OpeningGuard(pub Option<Arc<CloseState>>);
impl Drop for OpeningGuard {
    fn drop(&mut self) {
        if let Some(state) = self.0.take() {
            state.close();
        }
    }
}
