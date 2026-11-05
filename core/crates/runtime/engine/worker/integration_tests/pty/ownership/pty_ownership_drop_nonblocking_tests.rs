use super::*;
use crate::process::{CapturedProcessTree, ProcessIdentity};
use std::path::Path;
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const CHILD_EXECUTABLE: &str = "/bin/sleep";
const CHILD_DURATION: &str = "30";
const DROP_RETURN_TIMEOUT: Duration = Duration::from_secs(2);
const CHILD_REAP_TIMEOUT: Duration = Duration::from_secs(10);
const CHILD_REAP_INTERVAL: Duration = Duration::from_millis(5);
const EVENT_CHANNEL_CAPACITY: usize = 1;
const TEST_SPAN_ID: u64 = 1;
const DROP_BLOCKED_DETAIL: &str = "PTY ownership Drop entered a blocking tracing subscriber";
const DIRECT_DROP_BLOCKED_DETAIL: &str =
    "PTY ownership Drop waited for a full primary or residual reservation retirement";
const RETIREMENT_ENTRY_MISSING_DETAIL: &str =
    "background emergency escrow owner did not enter reservation retirement";
const DROP_THREAD_PANIC_DETAIL: &str = "PTY ownership Drop thread panicked";

struct BlockingSubscriber {
    entered: SyncSender<()>,
    release: Arc<ReleaseGate>,
}

#[derive(Default)]
struct ReleaseGate {
    released: Mutex<bool>,
    changed: Condvar,
}

impl ReleaseGate {
    fn wait(&self) {
        let mut released = self.released.lock().unwrap();
        while !*released {
            released = self.changed.wait(released).unwrap();
        }
    }

    fn open(&self) {
        *self.released.lock().unwrap() = true;
        self.changed.notify_all();
    }
}

impl Subscriber for BlockingSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _attributes: &Attributes<'_>) -> Id {
        Id::from_u64(TEST_SPAN_ID)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}
    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {
        let _ = self.entered.try_send(());
        self.release.wait();
    }

    fn enter(&self, _span: &Id) {}
    fn exit(&self, _span: &Id) {}
}

#[test]
fn ownership_drop_never_enters_blocking_tracing_subscriber() {
    let child = crate::process::spawn_detached(
        Path::new(CHILD_EXECUTABLE),
        &[CHILD_DURATION.into()],
        crate::process::DetachedStandardError::Null,
    )
    .unwrap();
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let ownership =
        PtyOwnership::new_for_test(Box::new(child), tree, reserve_fault_reaper().unwrap()).unwrap();
    let (entered_sender, entered_receiver) = mpsc::sync_channel(EVENT_CHANNEL_CAPACITY);
    let (done_sender, done_receiver) = mpsc::sync_channel(EVENT_CHANNEL_CAPACITY);
    let release = Arc::new(ReleaseGate::default());
    let subscriber = BlockingSubscriber {
        entered: entered_sender,
        release: Arc::clone(&release),
    };
    let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
    let dropper = std::thread::spawn(move || {
        tracing::subscriber::with_default(subscriber, || drop(ownership));
        done_sender.send(()).unwrap();
    });

    let returned = done_receiver.recv_timeout(DROP_RETURN_TIMEOUT);
    let subscriber_entered = entered_receiver.try_recv().is_ok();
    release.open();
    dropper.join().unwrap();

    assert!(returned.is_ok(), "{DROP_BLOCKED_DETAIL}");
    assert!(!subscriber_entered, "{DROP_BLOCKED_DETAIL}");
    wait_until_gone(root);
}

#[test]
fn ownership_drop_bypasses_full_primary_and_residual_retirement() {
    let (mut reaper, held_primary) =
        crate::worker::pty_fault_reaper::FaultReaperReservation::reserve_held_for_test().unwrap();
    reaper.fill_primary_for_test();
    let retirement = reaper.block_retirement_for_test();
    let child = crate::process::spawn_detached(
        Path::new(CHILD_EXECUTABLE),
        &[CHILD_DURATION.into()],
        crate::process::DetachedStandardError::Null,
    )
    .unwrap();
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let ownership = PtyOwnership::new_for_test(Box::new(child), tree, reaper).unwrap();
    let (done_sender, done_receiver) = mpsc::sync_channel(EVENT_CHANNEL_CAPACITY);
    let dropper = std::thread::spawn(move || {
        drop(ownership);
        done_sender.send(()).unwrap();
    });

    let initial_completion = done_receiver.recv_timeout(DROP_RETURN_TIMEOUT);
    let returned_before_release = initial_completion.is_ok();
    let retirement_entered = retirement.wait_until_entered();
    retirement.release();
    drop(held_primary);
    let dropper_finished = match initial_completion {
        Ok(()) | Err(RecvTimeoutError::Disconnected) => true,
        Err(RecvTimeoutError::Timeout) => !matches!(
            done_receiver.recv_timeout(DROP_RETURN_TIMEOUT),
            Err(RecvTimeoutError::Timeout)
        ),
    };
    let dropper_joined = dropper_finished && dropper.join().is_ok();

    assert!(returned_before_release, "{DIRECT_DROP_BLOCKED_DETAIL}");
    assert!(retirement_entered, "{RETIREMENT_ENTRY_MISSING_DETAIL}");
    assert!(dropper_joined, "{DROP_THREAD_PANIC_DETAIL}");
    wait_until_gone(root);
}

fn wait_until_gone(root: ProcessIdentity) {
    let deadline = Instant::now() + CHILD_REAP_TIMEOUT;
    while root.matches() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(CHILD_REAP_INTERVAL);
    }
}
