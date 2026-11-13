use std::sync::atomic::{AtomicBool, Ordering};

const PANIC_DETAIL: &str = "injected crash after durable broker Running phase";
static PANIC_AFTER_RUNNING: AtomicBool = AtomicBool::new(false);

pub(super) struct PanicAfterRunning;

pub(super) fn install() -> PanicAfterRunning {
    assert!(!PANIC_AFTER_RUNNING.swap(true, Ordering::SeqCst));
    PanicAfterRunning
}

pub(super) fn after_running_persist() {
    if PANIC_AFTER_RUNNING.swap(false, Ordering::SeqCst) {
        panic!("{PANIC_DETAIL}");
    }
}

impl Drop for PanicAfterRunning {
    fn drop(&mut self) {
        PANIC_AFTER_RUNNING.store(false, Ordering::SeqCst);
    }
}
