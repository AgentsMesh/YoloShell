use crate::host::tests::*;
use super::*;

use std::sync::atomic::{AtomicUsize as JoinReportAtomicUsize, Ordering as JoinReportOrdering};

const JOIN_REPORT_EXPECTED_PANIC_EVENTS: usize = 1;
const JOIN_REPORT_TASK_NOT_JOINED: &str = "ingress task did not finish joining";
const JOIN_REPORT_TASK_NOT_STARTED: &str = "ingress cancellation task did not start";
const JOIN_REPORT_TEST_PANIC: &str = "injected ingress task panic";
const JOIN_REPORT_TEST_SPAN_ID: u64 = 1;

struct JoinReportSubscriber(Arc<JoinReportAtomicUsize>);

impl tracing::Subscriber for JoinReportSubscriber {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(JOIN_REPORT_TEST_SPAN_ID)
    }

    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        if *event.metadata().level() == tracing::Level::ERROR {
            self.0.fetch_add(
                JOIN_REPORT_EXPECTED_PANIC_EVENTS,
                JoinReportOrdering::SeqCst,
            );
        }
    }

    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

#[tokio::test]
async fn successful_idle_shutdown_join_converges_without_a_panic_report() {
    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let events = Arc::new(JoinReportAtomicUsize::default());
    let _subscriber = tracing::subscriber::set_default(JoinReportSubscriber(Arc::clone(&events)));
    let mut tasks = tokio::task::JoinSet::new();
    tasks.spawn(async {});

    let joined = tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, tasks.join_next())
        .await
        .expect(JOIN_REPORT_TASK_NOT_JOINED);
    assert!(matches!(joined.as_ref(), Some(Ok(()))));
    report_joined_task_panic(joined, HOST_IDLE_SHUTDOWN_JOIN_PANIC_LOG);

    assert!(tasks.is_empty());
    assert_eq!(events.load(JoinReportOrdering::SeqCst), usize::default());
}

#[tokio::test]
async fn cancelled_idle_shutdown_join_converges_without_a_panic_report() {
    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let events = Arc::new(JoinReportAtomicUsize::default());
    let _subscriber = tracing::subscriber::set_default(JoinReportSubscriber(Arc::clone(&events)));
    let mut tasks = tokio::task::JoinSet::new();
    let (started, ready) = tokio::sync::oneshot::channel();
    tasks.spawn(async move {
        let _ = started.send(());
        std::future::pending::<()>().await;
    });
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, ready)
        .await
        .expect(JOIN_REPORT_TASK_NOT_STARTED)
        .expect(JOIN_REPORT_TASK_NOT_STARTED);

    tasks.abort_all();
    let joined = tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, tasks.join_next())
        .await
        .expect(JOIN_REPORT_TASK_NOT_JOINED);
    assert!(matches!(joined.as_ref(), Some(Err(error)) if error.is_cancelled()));
    report_joined_task_panic(joined, HOST_IDLE_SHUTDOWN_JOIN_PANIC_LOG);

    assert!(tasks.is_empty());
    assert_eq!(events.load(JoinReportOrdering::SeqCst), usize::default());
}

#[tokio::test]
async fn panicked_idle_shutdown_join_emits_one_error_report() {
    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let events = Arc::new(JoinReportAtomicUsize::default());
    let _subscriber = tracing::subscriber::set_default(JoinReportSubscriber(Arc::clone(&events)));
    let mut tasks = tokio::task::JoinSet::new();
    tasks.spawn(async { panic!("{JOIN_REPORT_TEST_PANIC}") });

    let joined = tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, tasks.join_next())
        .await
        .expect(JOIN_REPORT_TASK_NOT_JOINED);
    assert!(matches!(joined.as_ref(), Some(Err(error)) if error.is_panic()));
    report_joined_task_panic(joined, HOST_IDLE_SHUTDOWN_JOIN_PANIC_LOG);

    assert!(tasks.is_empty());
    assert_eq!(
        events.load(JoinReportOrdering::SeqCst),
        JOIN_REPORT_EXPECTED_PANIC_EVENTS
    );
}
