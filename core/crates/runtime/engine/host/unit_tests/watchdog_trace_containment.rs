use crate::host::tests::*;
use super::*;
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const WATCHDOG_SUBSCRIBER_PANIC: &str = "injected watchdog tracing subscriber panic";
const WATCHDOG_TEST_SPAN_ID: u64 = 1;

struct WatchdogPanicSubscriber;

impl Subscriber for WatchdogPanicSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(WATCHDOG_TEST_SPAN_ID)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {
        panic!("{WATCHDOG_SUBSCRIBER_PANIC}");
    }

    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

#[tokio::test(flavor = "current_thread")]
async fn watchdog_contains_tracing_subscriber_panic() {
    let (shutdown, receiver) = oneshot::channel();
    let mut shutdown = Some(shutdown);
    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let _subscriber = tracing::subscriber::set_default(WatchdogPanicSubscriber);

    run_watchdog(test_watchdog_schedule(), receiver, move || {
        let shutdown = shutdown.take();
        async move {
            if let Some(shutdown) = shutdown {
                let _ = shutdown.send(());
            }
        }
    })
    .await;
}
