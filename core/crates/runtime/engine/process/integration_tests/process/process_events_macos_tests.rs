use super::*;
use crate::process::{LiveProcessIdentity, ProcessIdentity, install_test_match_failure};
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

const SHELL_PATH: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const FORK_SCRIPT: &str = "printf ready; read gate; /usr/bin/true & wait; read hold";
const FORK_TRIGGER: &[u8] = b"fork\n";
const READY_MARKER: &[u8] = b"ready";
const EVENT_TIMEOUT: Duration = Duration::from_secs(2);
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(5);
const FORK_EVENT_MISSING_DETAIL: &str = "fork event was not observed";
const FORGED_GENERATION_INCREMENT: u64 = 1;
const SINGLE_REGISTERED_PROCESS: usize = 1;
const MATCH_FAILURE_DETAIL: &str = "injected watcher generation check failure";
const NON_IO_FAILURE_DETAIL: &str = "non-I/O event failure";

struct ForkFixture {
    child: Child,
    input: ChildStdin,
    live: LiveProcessIdentity,
}

impl ForkFixture {
    fn spawn() -> Self {
        let mut child = crate::process::spawn_waitable_test_child(
            Command::new(SHELL_PATH)
                .args([SHELL_COMMAND_ARGUMENT, FORK_SCRIPT])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null()),
        );
        let mut ready = vec![u8::default(); READY_MARKER.len()];
        child.stdout.take().unwrap().read_exact(&mut ready).unwrap();
        assert_eq!(ready, READY_MARKER);
        let durable = ProcessIdentity::capture(child.id()).unwrap();
        let live = LiveProcessIdentity::capture(durable).unwrap().unwrap();
        let input = child.stdin.take().unwrap();
        Self { child, input, live }
    }
}

impl Drop for ForkFixture {
    fn drop(&mut self) {
        let _ = self.live.signal(libc::SIGKILL);
        let _ = self.child.wait();
    }
}

#[test]
fn fork_event_invalidates_an_otherwise_idle_process_scope() {
    let mut fixture = ForkFixture::spawn();
    let mut events = MacosProcessEvents::new(fixture.live).unwrap();
    assert!(!events.changed().unwrap());

    fixture.input.write_all(FORK_TRIGGER).unwrap();
    let deadline = Instant::now() + EVENT_TIMEOUT;
    while !events.changed().unwrap() {
        assert!(Instant::now() < deadline, "{}", FORK_EVENT_MISSING_DETAIL);
        std::thread::sleep(EVENT_POLL_INTERVAL);
    }
}

#[test]
fn watcher_registration_rejects_a_forged_kernel_identity() {
    let live = current_identity();
    let forged = forged(live);

    assert!(MacosProcessEvents::new(forged).is_err());
    let exact_events = MacosProcessEvents::new(live).unwrap();
    assert_registered(&exact_events, live);
}

#[test]
fn reconciliation_selects_the_live_generation_in_either_input_order() {
    let live = current_identity();
    let stale = forged(live);

    for identities in [[live, stale], [stale, live]] {
        let mut events = MacosProcessEvents::new(live).unwrap();
        assert!(events.reconcile(identities).unwrap().registered.is_empty());
        assert_registered(&events, live);
    }
}

#[test]
fn dead_tombstone_is_not_registered() {
    let mut fixture = ForkFixture::spawn();
    let mut events = MacosProcessEvents::new(fixture.live).unwrap();
    assert!(fixture.live.signal(libc::SIGKILL).unwrap());
    fixture.child.wait().unwrap();

    assert!(
        events
            .reconcile([fixture.live])
            .unwrap()
            .registered
            .is_empty()
    );
    assert!(events.registered.is_empty());
    assert!(events.queue.is_some());
}

#[test]
fn replacement_generation_replaces_the_watcher_and_marks_the_gate_pending() {
    let live = current_identity();
    let stale = forged(live);
    let mut events = MacosProcessEvents::new(live).unwrap();
    events.registered.insert(live.pid(), stale);
    events.unresolved_forks.insert(stale.lineage_id());
    events.gone_fork_sources.insert(stale.lineage_id());

    assert_eq!(events.reconcile([stale, live]).unwrap().registered, [live]);
    assert_registered(&events, live);
}

#[test]
fn generation_check_failure_invalidates_the_entire_gate() {
    let live = current_identity();
    let mut events = MacosProcessEvents::new(live).unwrap();
    let _failure = install_test_match_failure(MATCH_FAILURE_DETAIL);

    assert!(events.reconcile([live]).is_err());
    assert!(events.registered.is_empty());
    assert!(events.queue.is_none());
}

#[test]
fn synthetic_batches_reject_invalid_events_and_drain_a_full_batch() {
    let live = current_identity();
    let mut events = MacosProcessEvents::new(live).unwrap();
    events.queue_invalid_test_read();
    assert!(events.changed().is_err());

    events.queue_valid_test_read(EVENT_BATCH_CAPACITY);
    events.queue_valid_test_read(usize::default());
    assert!(events.changed().unwrap());
}

#[test]
fn invalidated_unregister_and_error_classification_fail_closed() {
    let live = current_identity();
    let mut events = MacosProcessEvents::new(live).unwrap();
    events.queue.take();
    assert!(events.unregister(live.pid()).is_err());

    for code in [libc::ESRCH, libc::ENOENT] {
        let error = anyhow::Error::new(std::io::Error::from_raw_os_error(code));
        assert!(is_missing(&error));
    }
    let permission = anyhow::Error::new(std::io::Error::from_raw_os_error(libc::EPERM));
    assert!(!is_missing(&permission));
    assert!(!is_missing(&anyhow::anyhow!(NON_IO_FAILURE_DETAIL)));
}

#[test]
fn newly_registered_child_cannot_settle_coalesced_fork_uncertainty() {
    let parent = current_identity();
    let child = ForkFixture::spawn();
    let mut events = MacosProcessEvents::new(parent).unwrap();
    events.unresolved_forks.insert(parent.lineage_id());

    assert_eq!(
        events.reconcile([parent, child.live]).unwrap().registered,
        [child.live]
    );
    assert!(events.has_unresolved_fork());
}

fn current_identity() -> LiveProcessIdentity {
    let durable = ProcessIdentity::capture(std::process::id()).unwrap();
    LiveProcessIdentity::capture(durable).unwrap().unwrap()
}

fn forged(identity: LiveProcessIdentity) -> LiveProcessIdentity {
    identity.with_generation(
        identity
            .unique_id()
            .checked_add(FORGED_GENERATION_INCREMENT)
            .unwrap(),
    )
}

fn assert_registered(events: &MacosProcessEvents, live: LiveProcessIdentity) {
    assert_eq!(events.registered.len(), SINGLE_REGISTERED_PROCESS);
    assert_eq!(events.registered.get(&live.pid()), Some(&live));
}
