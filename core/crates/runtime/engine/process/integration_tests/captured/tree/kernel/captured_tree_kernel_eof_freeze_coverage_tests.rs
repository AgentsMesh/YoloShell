use super::FreezeClosureCandidate;
use super::exact_gate_fixture::ExactGateFixture;
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;

const TEST_CAPABILITY: &str = "kernel-eof-frozen-coverage-gate";
const EPOCH_CAPABILITY: &str = "kernel-eof-frozen-scheduling-epoch";
const PREMATURE_KERNEL_EOF_DETAIL: &str = "kernel EOF bypassed ordinary frozen scheduling";
const MISSING_KERNEL_EOF_DETAIL: &str =
    "completed ordinary frozen scheduling did not enable kernel EOF fallback";
const RETAINED_SCHEDULING_DETAIL: &str =
    "kernel EOF inspection discarded current released scheduling";
const STALE_SCHEDULING_DETAIL: &str = "stale released scheduling survived epoch invalidation";

#[test]
fn kernel_eof_requires_ordinary_frozen_scheduling_after_release() {
    let (_fixture, mut tree) = released_tree(TEST_CAPABILITY);
    let targets = tree.captured_targets();
    let authority = tree
        .captured_stopped_closure_authority(&targets)
        .unwrap()
        .unwrap();

    assert!(
        !tree.kernel_eof_stopped_closure_can_kill(&authority),
        "{PREMATURE_KERNEL_EOF_DETAIL}"
    );
    tree.capability_scheduling_complete = true;
    assert!(
        tree.kernel_eof_stopped_closure_can_kill(&authority),
        "{MISSING_KERNEL_EOF_DETAIL}"
    );
    assert!(matches!(
        super::exact_stopped_candidate(&mut tree).unwrap(),
        Some(FreezeClosureCandidate::KernelEof(_))
    ));
}

#[test]
fn released_scheduling_is_retained_only_within_the_current_epoch() {
    let (_fixture, mut tree) = released_tree(EPOCH_CAPABILITY);
    tree.capability_scheduling_complete = true;

    tree.refresh_for_kernel_eof().unwrap();
    assert!(
        tree.released_scheduling_coverage(),
        "{RETAINED_SCHEDULING_DETAIL}"
    );

    tree.begin_frozen_discovery();
    assert!(
        !tree.released_scheduling_coverage(),
        "{STALE_SCHEDULING_DETAIL}"
    );
    tree.capability_scheduling_complete = true;
    tree.invalidate_events();
    tree.refresh_for_kernel_eof().unwrap();
    assert!(
        !tree.released_scheduling_coverage(),
        "{STALE_SCHEDULING_DETAIL}"
    );
}

fn released_tree(capability: &str) -> (ExactGateFixture, crate::process::CapturedProcessTree) {
    let mut fixture = ExactGateFixture::spawn(capability, false);
    let mut tree = fixture.take_tree();
    tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    tree.capability_scan_pending = true;
    tree.capability_scheduling_complete = false;
    tree.lineage_integrity_lost = false;
    tree.pty_scan = Default::default();
    tree.pty_scan.mark_released_needs_scan_for_test();
    (fixture, tree)
}
