use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const AUTHORITY_OUTER_PANIC: &str = "injected wait-authority outer unwind";
const AUTHORITY_DISPLAY_PANIC: &str = "injected wait-authority Display panic";
const AUTHORITY_TEST_SPAN_ID: u64 = 1;

#[derive(Debug)]
struct HostileAuthorityError {
    error_dropped: Arc<AtomicBool>,
    source_payload_dropped: Arc<AtomicBool>,
    error_payload_dropped: Arc<AtomicBool>,
}

impl std::fmt::Display for HostileAuthorityError {
    fn fmt(&self, _formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        panic!("{AUTHORITY_DISPLAY_PANIC}");
    }
}

impl std::error::Error for HostileAuthorityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        std::panic::panic_any(HostileAuthorityPayload(Arc::clone(
            &self.source_payload_dropped,
        )));
    }
}

impl Drop for HostileAuthorityError {
    fn drop(&mut self) {
        self.error_dropped.store(true, Ordering::SeqCst);
        std::panic::panic_any(HostileAuthorityPayload(Arc::clone(
            &self.error_payload_dropped,
        )));
    }
}

struct HostileAuthorityPayload(Arc<AtomicBool>);

impl Drop for HostileAuthorityPayload {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
        panic!("injected wait-authority payload Drop panic");
    }
}

struct AuthorityPanicSubscriber {
    events: Arc<AtomicUsize>,
    payload_dropped: Arc<AtomicBool>,
}

impl Subscriber for AuthorityPanicSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(AUTHORITY_TEST_SPAN_ID)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {
        self.events.fetch_add(1, Ordering::SeqCst);
        std::panic::panic_any(HostileAuthorityPayload(Arc::clone(&self.payload_dropped)));
    }

    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

struct ClassifyAuthorityOnDrop<'a> {
    ownership: &'a mut FaultReap,
    error: Option<anyhow::Error>,
}

impl Drop for ClassifyAuthorityOnDrop<'_> {
    fn drop(&mut self) {
        live_authority::retain_or_lose_wait_authority(
            self.ownership,
            self.error.take().expect("hostile error is present"),
        );
    }
}

#[test]
fn wait_authority_classification_contains_hostile_error_during_outer_unwind() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let child = Box::new(child) as crate::process::OwnedPtyChild;
    let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
        .unwrap()
        .bind(child);
    let mut reaper = branch_reaper();
    reaper.custody_mut().authority = Some(authority);
    reaper.custody_mut().processes = Some(PtyProcessCustody::tracking(tree));
    reaper.completion().publish(ReapOutcome::waited(None));
    let flags = AuthorityContainmentFlags::new();
    let guard = ClassifyAuthorityOnDrop {
        ownership: &mut reaper,
        error: Some(flags.hostile_error()),
    };
    let subscriber = AuthorityPanicSubscriber {
        events: Arc::clone(&flags.trace_events),
        payload_dropped: Arc::clone(&flags.trace_payload_dropped),
    };

    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tracing::subscriber::with_default(subscriber, || {
            let _guard = guard;
            panic!("{AUTHORITY_OUTER_PANIC}");
        });
    }));

    assert_eq!(
        authority_panic_message(result.unwrap_err().as_ref()),
        Some(AUTHORITY_OUTER_PANIC)
    );
    assert!(reaper.custody().authority.is_some());
    assert!(reaper.lost_wait_custody.is_none());
    flags.assert_contained();
    assert!(root.signal(libc::SIGKILL).unwrap());
    assert!(wait_direct_child(root.pid).status.is_some());
}

struct AuthorityContainmentFlags {
    error_dropped: Arc<AtomicBool>,
    source_payload_dropped: Arc<AtomicBool>,
    error_payload_dropped: Arc<AtomicBool>,
    trace_events: Arc<AtomicUsize>,
    trace_payload_dropped: Arc<AtomicBool>,
}

impl AuthorityContainmentFlags {
    fn new() -> Self {
        Self {
            error_dropped: Arc::new(AtomicBool::default()),
            source_payload_dropped: Arc::new(AtomicBool::default()),
            error_payload_dropped: Arc::new(AtomicBool::default()),
            trace_events: Arc::new(AtomicUsize::default()),
            trace_payload_dropped: Arc::new(AtomicBool::default()),
        }
    }

    fn hostile_error(&self) -> anyhow::Error {
        anyhow::Error::new(HostileAuthorityError {
            error_dropped: Arc::clone(&self.error_dropped),
            source_payload_dropped: Arc::clone(&self.source_payload_dropped),
            error_payload_dropped: Arc::clone(&self.error_payload_dropped),
        })
    }

    fn assert_contained(&self) {
        assert!(self.error_dropped.load(Ordering::SeqCst));
        assert!(!self.source_payload_dropped.load(Ordering::SeqCst));
        assert!(!self.error_payload_dropped.load(Ordering::SeqCst));
        assert!(self.trace_events.load(Ordering::SeqCst) > 0);
        assert!(!self.trace_payload_dropped.load(Ordering::SeqCst));
    }
}

fn authority_panic_message(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}
