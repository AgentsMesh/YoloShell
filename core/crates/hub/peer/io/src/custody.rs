use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::task::{Context, Waker};

type CloseOnce = Box<dyn FnOnce() + Send + Sync>;

pub(crate) struct CloseState {
    closed: AtomicBool,
    callback: Mutex<Option<CloseOnce>>,
    readers: Mutex<Option<Waker>>,
    writers: Mutex<Option<Waker>>,
}

impl CloseState {
    pub fn new(close: impl FnOnce() + Send + Sync + 'static) -> Arc<Self> {
        Arc::new(Self {
            closed: AtomicBool::new(false),
            callback: Mutex::new(Some(Box::new(close))),
            readers: Mutex::new(None),
            writers: Mutex::new(None),
        })
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
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

impl IoCustody {
    pub fn close(&self) {
        self.0.close();
    }
}

impl Drop for IoCustody {
    fn drop(&mut self) {
        self.close();
    }
}
