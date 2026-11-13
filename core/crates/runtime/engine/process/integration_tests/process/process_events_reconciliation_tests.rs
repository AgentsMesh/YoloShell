use super::*;
use crate::process::ProcessIdentity;
use std::io::Write;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::time::{Duration, Instant};

const SHELL_PATH: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const FORK_EXIT_SCRIPT: &str = "read gate; /usr/bin/true & exit 0";
const FORK_EXIT_TRIGGER: &[u8] = b"fork\n";
const PROCESS_EXIT_TIMEOUT: Duration = Duration::from_secs(2);
const PROCESS_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(5);
const PROCESS_EXIT_TIMEOUT_DETAIL: &str = "fork source did not become an unreaped zombie";

struct ForkExitFixture {
    child: Child,
    input: Option<ChildStdin>,
    live: LiveProcessIdentity,
}

impl ForkExitFixture {
    fn spawn() -> Self {
        let mut child = crate::process::spawn_waitable_test_child(
            Command::new(SHELL_PATH)
                .args([SHELL_COMMAND_ARGUMENT, FORK_EXIT_SCRIPT])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null()),
        );
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        let live = LiveProcessIdentity::capture(identity).unwrap().unwrap();
        let input = child.stdin.take();
        Self { child, input, live }
    }

    fn trigger_and_wait_for_exit(&mut self) {
        self.input
            .take()
            .unwrap()
            .write_all(FORK_EXIT_TRIGGER)
            .unwrap();
        let deadline = Instant::now() + PROCESS_EXIT_TIMEOUT;
        while self.live.try_matches().unwrap() {
            assert!(Instant::now() < deadline, "{PROCESS_EXIT_TIMEOUT_DETAIL}");
            std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
        }
    }
}

impl Drop for ForkExitFixture {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn pending_reconciliation_still_drains_a_queued_fork_event() {
    let root = live_root();
    let mut events = ProcessEvents::new(root);
    events.pending_reconciliation = true;
    events.queue_test_fork(root);

    assert!(events.requires_reconciliation());
    assert!(events.has_unresolved_fork());
}

#[test]
fn post_inventory_fence_drains_events_even_when_a_watcher_audit_is_pending() {
    let root = live_root();
    let mut events = ProcessEvents::new(root);
    events.pending_reconciliation = true;
    events.queue_test_fork(root);

    assert!(!events.post_inventory_quiescent());
    assert!(events.has_unresolved_fork());
}

#[test]
fn event_read_fence_retains_existing_debt_but_rejects_a_fresh_fork() {
    let root = live_root();
    let mut events = ProcessEvents::new(root);
    events.pending_reconciliation = true;
    events.queue_quiescent_test_read();

    assert!(events.post_inventory_event_read_quiescent());
    assert!(events.reconciliation_pending_for_test());

    events.queue_test_fork(root);
    assert!(!events.post_inventory_event_read_quiescent());
    assert!(events.has_unresolved_fork());
    assert!(events.reconciliation_pending_for_test());
}

#[test]
fn gate_loss_requires_one_follow_up_dense_reconciliation() {
    let root = live_root();
    let mut events = ProcessEvents::new(root);
    events.fail_next_read();

    assert!(events.requires_reconciliation());
    assert!(!events.reconcile_inventory([root]));
    assert!(events.requires_reconciliation());
    assert!(events.reconcile_inventory([root]));
}

#[test]
fn captured_debt_merges_followup_and_invalidates_later_publication() {
    let root = live_root();
    let mut events = ProcessEvents::new(root);
    events.fail_next_read();
    assert!(events.requires_reconciliation());
    let debt = events.capture_reconciliation_debt();

    assert!(!events.reconcile_inventory([root]));
    assert!(events.reconcile_inventory([root]));
    assert!(events.publication_fence.is_some());

    events.merge_reconciliation_debt(debt);
    assert!(events.pending_reconciliation);
    assert!(events.reconciliation_requires_followup);
    assert!(events.publication_fence.is_none());
}

#[test]
fn pre_reconcile_drain_preserves_fork_exit_before_watcher_removal() {
    let root = live_root();
    let mut events = ProcessEvents::new(root);
    events.queue_test_fork_exit(root);

    let publication_quiescent = events.reconcile_inventory(std::iter::empty());

    assert_eq!(
        events.unresolved_fork_lineages_for_test(),
        [root.lineage_id()]
    );
    assert!(events.exact_closure_isolated(&[]));
    assert!(events.reconciliation_pending_for_test());
    assert!(!publication_quiescent);
    assert!(!events.post_inventory_quiescent());
}

#[test]
fn internal_reconcile_fence_preserves_fork_exit_before_watcher_removal() {
    let root = live_root();
    let mut events = ProcessEvents::new(root);
    events.queue_reconcile_test_fork_exit(root);

    let publication_quiescent = events.reconcile_inventory(std::iter::empty());

    assert_eq!(
        events.unresolved_fork_lineages_for_test(),
        [root.lineage_id()]
    );
    assert!(events.exact_closure_isolated(&[]));
    assert!(events.reconciliation_pending_for_test());
    assert!(!publication_quiescent);
}

#[test]
fn kernel_fork_exit_survives_reconciliation_of_an_unreaped_source() {
    let mut fixture = ForkExitFixture::spawn();
    let mut events = ProcessEvents::new(fixture.live);
    assert!(events.is_gated());
    fixture.trigger_and_wait_for_exit();

    let publication_quiescent = events.reconcile_inventory(std::iter::empty());

    assert_eq!(
        events.unresolved_fork_lineages_for_test(),
        [fixture.live.lineage_id()]
    );
    assert!(events.exact_closure_isolated(&[]));
    assert!(!publication_quiescent);
}

fn live_root() -> LiveProcessIdentity {
    let identity = ProcessIdentity::capture(std::process::id()).unwrap();
    LiveProcessIdentity::capture(identity).unwrap().unwrap()
}
