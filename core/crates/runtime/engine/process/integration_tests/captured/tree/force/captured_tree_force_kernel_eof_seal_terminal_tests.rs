use super::super::super::reopen_seal_terminal::ERROR_DETAIL_FALLBACK;
use super::*;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const DISPLAY_PANIC: &str = "injected kernel EOF invalidation error Display panic";
const DROP_PANIC: &str = "injected kernel EOF invalidation error Drop panic";
const ERROR_COUNT: usize = 2;
const INVALIDATION_CONSTRUCTION_PANICKED: &str =
    "kernel EOF invalidation escaped terminal error containment";

#[derive(Debug)]
struct AdversarialError {
    displays: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
}

impl fmt::Display for AdversarialError {
    fn fmt(&self, _formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.displays.fetch_add(1, Ordering::SeqCst);
        panic!("{DISPLAY_PANIC}");
    }
}

impl std::error::Error for AdversarialError {}

impl Drop for AdversarialError {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
        panic!("{DROP_PANIC}");
    }
}

#[test]
fn typed_invalidation_contains_source_and_cleanup_format_and_drop_panics() {
    let displays = Arc::new(AtomicUsize::default());
    let drops = Arc::new(AtomicUsize::default());
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        kernel_eof_output_invalidated(
            adversarial_error(&displays, &drops),
            Some(adversarial_error(&displays, &drops)),
        )
    }));

    let error = outcome.expect(INVALIDATION_CONSTRUCTION_PANICKED);
    assert!(FrozenProcessTree::kernel_eof_error_invalidates_output(
        &error
    ));
    let detail = format!("{error:#}");
    assert!(detail.contains(REOPEN_SEAL_SOURCE_FORMATTING_PANIC_DETAIL));
    assert!(detail.contains(ERROR_DETAIL_FALLBACK));
    assert_eq!(displays.load(Ordering::SeqCst), ERROR_COUNT);
    assert_eq!(drops.load(Ordering::SeqCst), ERROR_COUNT);
}

fn adversarial_error(displays: &Arc<AtomicUsize>, drops: &Arc<AtomicUsize>) -> anyhow::Error {
    anyhow::Error::new(AdversarialError {
        displays: Arc::clone(displays),
        drops: Arc::clone(drops),
    })
}
