use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const CUSTODY_DROP_PANIC: &str = "injected retired secondary-custody Drop panic";
const POST_SUPERSEDE_FAULT: &str = "injected fault after supersede commit";
const OUTER_UNWIND: &str = "injected outer secondary-custody fail-stop unwind";
const SUBSCRIBER_PAYLOAD_DROP_PANIC: &str =
    "injected secondary-custody subscriber payload Drop panic";
const TEST_SPAN_ID: u64 = 1;

struct PanicSubscriber {
    events: Arc<AtomicUsize>,
    payload_dropped: Arc<AtomicBool>,
}

impl Subscriber for PanicSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(TEST_SPAN_ID)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {
        self.events.fetch_add(1, Ordering::SeqCst);
        std::panic::panic_any(SubscriberPayload(Arc::clone(&self.payload_dropped)));
    }

    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

struct SubscriberPayload(Arc<AtomicBool>);

impl Drop for SubscriberPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
        panic!("{SUBSCRIBER_PAYLOAD_DROP_PANIC}");
    }
}

#[test]
fn cancellation_fail_stop_survives_subscriber_panic_during_outer_unwind() {
    let directory = tempfile::tempdir().unwrap();
    let host = Host::open(crate::host::HostConfig::new(directory.path())).unwrap();
    let mut fail_closed =
        super::exchange_guard::PostPreparedFailStop::new(host.request_drain.clone());
    fail_closed.arm();
    let events = Arc::new(AtomicUsize::default());
    let payload_dropped = Arc::new(AtomicBool::default());
    let subscriber = PanicSubscriber {
        events: Arc::clone(&events),
        payload_dropped: Arc::clone(&payload_dropped),
    };
    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        tracing::subscriber::with_default(subscriber, || {
            let _fail_closed = fail_closed;
            panic!("{OUTER_UNWIND}");
        });
    }));

    assert_eq!(
        panic_message(outcome.unwrap_err().as_ref()),
        Some(OUTER_UNWIND)
    );
    assert_eq!(events.load(Ordering::SeqCst), 1);
    assert!(payload_dropped.load(Ordering::Acquire));
    assert!(host.request_drain.fail_stop_detail().is_some());
}

struct PanicCustodyDrop(Arc<AtomicBool>);

impl Drop for PanicCustodyDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
        panic!("{CUSTODY_DROP_PANIC}");
    }
}

#[test]
fn retired_platform_drop_panic_is_contained_and_fail_stops() {
    let directory = tempfile::tempdir().unwrap();
    let host = Host::open(crate::host::HostConfig::new(directory.path())).unwrap();
    let dropped = Arc::new(AtomicBool::default());

    super::containment::discard_after_retirement(
        PanicCustodyDrop(Arc::clone(&dropped)),
        &host.request_drain,
    );

    assert!(dropped.load(Ordering::Acquire));
    assert!(host.request_drain.fail_stop_detail().is_some());
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn committed_supersede_rejects_late_monitor_fault_transitions() {
    let (fixture, lease, server, finish) = super::release_commit_tests::ready_fixture().await;
    lease.commit_superseded().unwrap();

    super::failure::fail_closed(Arc::downgrade(&lease), POST_SUPERSEDE_FAULT.to_owned());
    super::failure::respond(
        Arc::downgrade(&lease),
        super::failure::CustodyMonitorFault::failed(POST_SUPERSEDE_FAULT),
    )
    .await;

    assert_eq!(lease.state(), SecondaryCustodyState::Superseded);
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
    finish.send(()).unwrap();
    server.await.unwrap();
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}
