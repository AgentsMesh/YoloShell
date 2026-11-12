use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const ERROR_DISPLAY_PANIC: &str = "injected startup cleanup error Display panic";
const ERROR_DROP_PANIC: &str = "injected startup cleanup error Drop panic";
const DROP_ERROR_DETAIL: &str = "startup cleanup error with panicking Drop";
const RETAINED_ERROR_DETAIL: &str = "retained startup cleanup error";

#[derive(Debug)]
struct PanicDisplayError;

impl fmt::Display for PanicDisplayError {
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        panic!("{ERROR_DISPLAY_PANIC}");
    }
}

impl std::error::Error for PanicDisplayError {}

#[derive(Debug)]
struct PanicDropError(Arc<AtomicBool>);

impl fmt::Display for PanicDropError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(DROP_ERROR_DETAIL)
    }
}

impl std::error::Error for PanicDropError {}

impl Drop for PanicDropError {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
        panic!("{ERROR_DROP_PANIC}");
    }
}

#[test]
fn startup_cleanup_error_display_panic_cannot_escape_drop_containment() {
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        super::contain_drop_cleanup(|| Err(anyhow::Error::new(PanicDisplayError)));
    }));

    assert!(outcome.is_ok());
}

#[test]
fn startup_cleanup_error_drop_panic_cannot_escape_drop_containment() {
    let dropped = Arc::new(AtomicBool::new(false));
    let error_dropped = Arc::clone(&dropped);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        super::contain_drop_cleanup(|| Err(anyhow::Error::new(PanicDropError(error_dropped))));
    }));

    assert!(outcome.is_ok());
    assert!(dropped.load(Ordering::Acquire));
}

#[test]
fn later_cleanup_error_drop_panic_cannot_replace_the_retained_error() {
    let dropped = Arc::new(AtomicBool::default());
    let observed = Arc::clone(&dropped);
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let mut failure = Some(anyhow::anyhow!(RETAINED_ERROR_DETAIL));
        super::cleanup::remember(&mut failure, anyhow::Error::new(PanicDropError(dropped)));
        assert!(failure.unwrap().to_string().contains(RETAINED_ERROR_DETAIL));
    }));

    assert!(outcome.is_ok());
    assert!(observed.load(Ordering::Acquire));
}
