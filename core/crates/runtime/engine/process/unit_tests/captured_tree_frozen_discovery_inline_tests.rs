use super::CapturedProcessTree;
use crate::process::PtySessionProof;
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::inventory_test_support::TestProcess;
use portable_pty::{PtySize, native_pty_system};

const TEST_CAPABILITY: &str = "retained-bootstrap-coverage";
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn bootstrap_fence_retained_coverage_preserves_a_later_fork_for_acknowledgement() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let root = TestProcess::spawn();
    let mut proof =
        PtySessionProof::capture_bootstrap_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned())
            .unwrap();
    proof.retain_bootstrap_capability_coverage();
    let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
    let live_root = tree.live_root();
    let root_exit = tree.root_exit_proof_for_test(root.identity);
    tree.mark_root_exited_unreaped(root_exit).unwrap();
    tree.events.inject_unresolved_fork(live_root.clone());

    tree.refresh().unwrap();

    assert!(tree.events.has_unresolved_fork());
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);
    assert!(tree.acknowledge_frozen_forks(&[]));
    assert!(tree.events.has_unresolved_fork());
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);
    assert!(
        tree.pty_proof
            .as_ref()
            .is_some_and(|proof| { !proof.bootstrap_capability_coverage_is_retained() })
    );
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TERMINAL_ROWS,
        cols: TERMINAL_COLUMNS,
        pixel_width: TERMINAL_PIXEL_SIZE,
        pixel_height: TERMINAL_PIXEL_SIZE,
    }
}
