use super::terminal;
use crate::process::resilience_cleanup_containment_test_support::{
    adversarial_error, panic_message, panic_subscriber,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const REPORT_MESSAGE: &str = "resilience cleanup terminal containment test";
const OUTER_PANIC: &str = "injected resilience cleanup outer unwind";

struct ReportOnDrop(Option<anyhow::Error>);

impl Drop for ReportOnDrop {
    fn drop(&mut self) {
        terminal::report_error(self.0.take().unwrap(), REPORT_MESSAGE);
    }
}

#[test]
fn report_contains_format_drop_subscriber_and_payload_panics_during_outer_unwind() {
    let displays = Arc::new(AtomicUsize::default());
    let drops = Arc::new(AtomicUsize::default());
    let (subscriber, events, payload_drops) = panic_subscriber();
    let _tracing_test = yolo_application_test_fixtures::tracing::serialize_subscriber_tests();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tracing::subscriber::with_default(subscriber, || {
            let _report = ReportOnDrop(Some(adversarial_error(&displays, &drops)));
            panic!("{OUTER_PANIC}");
        });
    }));

    assert_eq!(
        panic_message(outcome.unwrap_err().as_ref()),
        Some(OUTER_PANIC)
    );
    assert_eq!(displays.load(Ordering::SeqCst), 1);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(events.load(Ordering::SeqCst), 1);
    assert_eq!(payload_drops.load(Ordering::SeqCst), 1);
}
