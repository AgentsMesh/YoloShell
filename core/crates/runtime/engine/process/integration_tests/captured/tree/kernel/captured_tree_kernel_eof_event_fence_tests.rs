use super::exact_gate_fixture::ExactGateFixture;
use crate::process::CapturedProcessTree;
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;

const EXPIRED_AUDIT_CAPABILITY: &str = "kernel-eof-expired-audit";
const STICKY_DEBT_CAPABILITY: &str = "kernel-eof-sticky-debt";

#[test]
fn expired_watcher_audit_denies_kernel_eof_before_inventory_publication() {
    let (_fixture, mut tree) = kernel_eof_tree(EXPIRED_AUDIT_CAPABILITY);
    tree.expire_event_audit();

    assert!(!kernel_eof_authorized(&mut tree));
    assert!(tree.events.reconciliation_pending_for_test());
}

#[test]
fn observed_reconciliation_debt_stays_sticky_until_inventory_publication() {
    let (_fixture, mut tree) = kernel_eof_tree(STICKY_DEBT_CAPABILITY);
    tree.invalidate_events();

    assert!(tree.events.requires_reconciliation());
    assert!(tree.events.reconciliation_pending_for_test());
    assert!(!kernel_eof_authorized(&mut tree));

    let identities = tree.captured.keys().cloned().collect::<Vec<_>>();
    assert!(tree.events.reconcile_inventory(identities));
    assert!(!tree.events.reconciliation_pending_for_test());
}

fn kernel_eof_tree(capability: &str) -> (ExactGateFixture, CapturedProcessTree) {
    let mut fixture = ExactGateFixture::spawn(capability, false);
    let mut tree = fixture.take_tree();
    tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    tree.capability_scan_pending = true;
    tree.lineage_integrity_lost = false;
    tree.pty_scan = Default::default();
    tree.pty_scan.mark_released_needs_scan_for_test();
    (fixture, tree)
}

fn kernel_eof_authorized(tree: &mut CapturedProcessTree) -> bool {
    let targets = tree.captured_targets();
    let authority = tree
        .captured_stopped_closure_authority(&targets)
        .unwrap()
        .unwrap();
    tree.kernel_eof_stopped_closure_can_kill(&authority)
}
