use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT};
use crate::process::inventory_test_support::TestProcess;
use crate::process::{CapturedProcessTree, FreezePoll};
use std::time::Instant;

const NON_PTY_FREEZE_TIMEOUT_DETAIL: &str =
    "ordinary force remained blocked on an inactive discovery epoch";

#[test]
fn non_pty_force_completes_without_starting_frozen_discovery() {
    let root = TestProcess::spawn();
    let mut freezing = CapturedProcessTree::capture(root.identity)
        .unwrap()
        .begin_force();
    let deadline = Instant::now() + TEST_TIMEOUT;

    loop {
        assert_eq!(freezing.tree.discovery_epoch, DiscoveryScanEpoch::Inactive);
        match freezing.poll().unwrap() {
            FreezePoll::Frozen(frozen) => {
                assert_eq!(frozen.tree.discovery_epoch, DiscoveryScanEpoch::Inactive);
                break;
            }
            FreezePoll::Pending(next) => freezing = next,
        }
        assert!(Instant::now() < deadline, "{NON_PTY_FREEZE_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}
