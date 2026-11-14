use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Debug)]
pub(super) struct ProcessWaitSignal {
    changed: Condvar,
    generation: Mutex<Arc<()>>,
}

pub(super) struct ProcessWaitToken {
    generation: Arc<()>,
    signal: Arc<ProcessWaitSignal>,
}

impl ProcessWaitSignal {
    pub(super) fn token(self: &Arc<Self>) -> ProcessWaitToken {
        ProcessWaitToken {
            generation: self
                .generation
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
            signal: Arc::clone(self),
        }
    }

    pub(super) fn notify_all(&self) {
        *self
            .generation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Arc::new(());
        self.changed.notify_all();
    }
}

impl Default for ProcessWaitSignal {
    fn default() -> Self {
        Self {
            changed: Condvar::new(),
            generation: Mutex::new(Arc::new(())),
        }
    }
}

impl ProcessWaitToken {
    pub(super) fn wait(self) {
        let generation = self
            .signal
            .generation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        drop(
            self.signal
                .changed
                .wait_while(generation, |current| Arc::ptr_eq(current, &self.generation))
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
    }

    pub(super) fn wait_timeout(self, timeout: Duration) {
        let generation = self
            .signal
            .generation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        drop(
            self.signal
                .changed
                .wait_timeout_while(generation, timeout, |current| {
                    Arc::ptr_eq(current, &self.generation)
                })
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        );
    }
}
