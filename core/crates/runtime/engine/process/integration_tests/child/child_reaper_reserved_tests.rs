use super::*;
use crate::process::{DetachedStandardError, spawn_detached};
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const DISPLAY_PANIC: &str = "injected reserved settlement error Display panic";
const DROP_PANIC: &str = "injected reserved settlement error Drop panic";
const PAYLOAD_DROP_PANIC: &str = "injected reserved settlement subscriber payload Drop panic";
const REPORT_MESSAGE: &str = "test reserved settlement capture failure";
const TEST_EXECUTABLE: &str = crate::test_executables::TRUE;
const TEST_SPAN_ID: u64 = 1;
const WAITID_OPTIONS: libc::c_int = libc::WEXITED | libc::WNOHANG | libc::WNOWAIT;
const WAITID_SUCCESS: libc::c_int = 0;
const NO_EXIT_PID: libc::pid_t = 0;
const WAITPID_OPTIONS: libc::c_int = libc::WNOHANG;
const WAITPID_FAILURE: libc::pid_t = -1;
const OBSERVATION_INTERVAL: Duration = Duration::from_millis(5);
const TEST_TIMEOUT: Duration = Duration::from_secs(2);
const EXIT_TIMEOUT_DETAIL: &str = "test child did not exit before capture";

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

struct PanicSubscriber {
    events: Arc<AtomicUsize>,
    payload_drops: Arc<AtomicUsize>,
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
        std::panic::panic_any(PanicPayload(Arc::clone(&self.payload_drops)));
    }
    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

struct PanicPayload(Arc<AtomicUsize>);

impl Drop for PanicPayload {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
        panic!("{PAYLOAD_DROP_PANIC}");
    }
}

#[test]
fn capture_error_reporter_contains_format_drop_and_subscriber_panics() {
    let displays = Arc::new(AtomicUsize::default());
    let drops = Arc::new(AtomicUsize::default());
    let (subscriber, events, payload_drops) = panic_subscriber();
    let error = AdversarialError {
        displays: Arc::clone(&displays),
        drops: Arc::clone(&drops),
    };
    let _tracing_test = yolo_application_test_fixtures::tracing::serialize_subscriber_tests();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tracing::subscriber::with_default(subscriber, || {
            containment::report_error(anyhow::Error::new(error), REPORT_MESSAGE);
        });
    }));

    assert!(outcome.is_ok());
    assert_eq!(displays.load(Ordering::SeqCst), 1);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(events.load(Ordering::SeqCst), 1);
    assert_eq!(payload_drops.load(Ordering::SeqCst), 1);
}

#[test]
fn capture_failure_subscriber_panic_preserves_exact_wait_custody() {
    let reaper = local_reaper();
    let reservation = reaper.reserve().unwrap();
    let child =
        spawn_detached(Path::new(TEST_EXECUTABLE), &[], DetachedStandardError::Null).unwrap();
    let process_id = child.id();
    let exited = wait_until_exited_without_reaping(process_id);
    let (subscriber, events, payload_drops) = panic_subscriber();
    let _tracing_test = yolo_application_test_fixtures::tracing::serialize_subscriber_tests();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tracing::subscriber::with_default(subscriber, || {
            reserved::ReservedDetachedChildSettlement::new(reservation).terminate(child);
        });
    }));

    exited.unwrap();
    assert!(outcome.is_ok());
    assert_eq!(events.load(Ordering::SeqCst), 1);
    assert_eq!(payload_drops.load(Ordering::SeqCst), 1);
    reaper
        .wait_for_termination_delivery(Instant::now() + TEST_TIMEOUT)
        .unwrap();
    assert_exact_reap(process_id);
    assert_eq!(lock_queue(&reaper.shared).custody_count, usize::default());
}

fn panic_subscriber() -> (PanicSubscriber, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let events = Arc::new(AtomicUsize::default());
    let payload_drops = Arc::new(AtomicUsize::default());
    (
        PanicSubscriber {
            events: Arc::clone(&events),
            payload_drops: Arc::clone(&payload_drops),
        },
        events,
        payload_drops,
    )
}

fn local_reaper() -> DetachedChildReaper {
    DetachedChildReaper::start_with(|shared| {
        std::thread::spawn(move || worker::run(shared));
        Ok(())
    })
    .unwrap()
}

fn wait_until_exited_without_reaping(process_id: u32) -> std::io::Result<()> {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while Instant::now() < deadline {
        let mut information = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                process_id as libc::id_t,
                information.as_mut_ptr(),
                WAITID_OPTIONS,
            )
        };
        if result != WAITID_SUCCESS {
            let error = std::io::Error::last_os_error();
            if error.kind() != std::io::ErrorKind::Interrupted {
                return Err(error);
            }
        } else if unsafe { information.assume_init().si_pid() } != NO_EXIT_PID {
            return Ok(());
        }
        std::thread::sleep(OBSERVATION_INTERVAL);
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        EXIT_TIMEOUT_DETAIL,
    ))
}

fn assert_exact_reap(process_id: u32) {
    let waited = unsafe {
        libc::waitpid(
            process_id as libc::pid_t,
            std::ptr::null_mut(),
            WAITPID_OPTIONS,
        )
    };
    assert_eq!(waited, WAITPID_FAILURE);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}
