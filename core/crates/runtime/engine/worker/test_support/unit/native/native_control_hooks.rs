use std::sync::Mutex;

#[derive(Default)]
pub(in crate::worker) struct NativeControlTestHooks {
    after_reserve: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    before_signal: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}

impl NativeControlTestHooks {
    pub(in crate::worker) fn after_next_reserve(&self, callback: impl FnOnce() + Send + 'static) {
        assert!(
            self.after_reserve
                .lock()
                .unwrap()
                .replace(Box::new(callback))
                .is_none()
        );
    }

    pub(in crate::worker) fn before_next_signal(&self, callback: impl FnOnce() + Send + 'static) {
        assert!(
            self.before_signal
                .lock()
                .unwrap()
                .replace(Box::new(callback))
                .is_none()
        );
    }

    pub(in crate::worker) fn run_after_reserve(&self) {
        let callback = self.after_reserve.lock().unwrap().take();
        if let Some(callback) = callback {
            callback();
        }
    }

    pub(in crate::worker) fn run_before_signal(&self) {
        let callback = self.before_signal.lock().unwrap().take();
        if let Some(callback) = callback {
            callback();
        }
    }
}
