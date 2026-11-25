use super::super::reopen_seal_terminal::ERROR_DETAIL_FALLBACK;
use super::*;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const CLEANUP_DISPLAY_PANIC: &str = "injected transaction cleanup error Display panic";
const CLEANUP_DROP_PANIC: &str = "injected transaction cleanup error Drop panic";
const ORIGINAL_TRANSACTION_FAILURE: &str = "injected original transaction failure";
const TRANSACTION_ERROR_REPLACED: &str = "cleanup replaced the original transaction error";
const TRANSACTION_ERROR_PANICKED: &str = "cleanup escaped the normal transaction error boundary";

#[derive(Debug)]
struct AdversarialCleanupError {
    displays: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
}

impl fmt::Display for AdversarialCleanupError {
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.displays.fetch_add(1, Ordering::SeqCst);
        panic!("{CLEANUP_DISPLAY_PANIC}");
    }
}

impl std::error::Error for AdversarialCleanupError {}

impl Drop for AdversarialCleanupError {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
        panic!("{CLEANUP_DROP_PANIC}");
    }
}

#[test]
fn normal_error_retains_source_across_cleanup_format_and_drop_panics() {
    let displays = Arc::new(AtomicUsize::default());
    let drops = Arc::new(AtomicUsize::default());
    let cleanup = Error::new(AdversarialCleanupError {
        displays: Arc::clone(&displays),
        drops: Arc::clone(&drops),
    });

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        failed_transaction_error(Error::msg(ORIGINAL_TRANSACTION_FAILURE), Err(cleanup))
    }));

    let error = outcome.expect(TRANSACTION_ERROR_PANICKED);
    let detail = format!("{error:#}");
    assert!(
        detail.contains(ORIGINAL_TRANSACTION_FAILURE),
        "{TRANSACTION_ERROR_REPLACED}: {detail}"
    );
    assert!(detail.contains(ERROR_DETAIL_FALLBACK));
    assert_eq!(displays.load(Ordering::SeqCst), 1);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
