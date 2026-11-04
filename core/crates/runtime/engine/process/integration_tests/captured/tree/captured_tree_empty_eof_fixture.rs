use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::inventory_test_support::TestProcess;
use crate::process::{
    CapturedProcessTree, FreezingProcessTree, LiveProcessIdentity, PtySessionProof,
};
use portable_pty::{PtyPair, PtySize, native_pty_system};

const TEST_CAPABILITY: &str = "empty-eof-authority";
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_PIXEL_SIZE: u16 = 0;

pub(super) struct EmptyEofFixture {
    pub(super) tree: CapturedProcessTree,
    _pty: PtyPair,
}

impl EmptyEofFixture {
    pub(super) fn exact_gone_source() -> Self {
        let pty = native_pty_system().openpty(pty_size()).unwrap();
        let mut root = TestProcess::spawn();
        let proof = PtySessionProof::capture_bound(pty.master.as_ref(), TEST_CAPABILITY.to_owned())
            .unwrap();
        let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
        let root_live = tree.live_root();
        tree.events.inject_unresolved_fork(root_live);
        root.terminate();
        reconcile_root_exit_publications(&mut tree);
        assert!(tree.events.exact_closure_isolated(&[]));

        tree.captured.clear();
        tree.retiring.clear();
        tree.discovery_epoch = DiscoveryScanEpoch::Covered;
        tree.capability_scan_pending = true;
        tree.lineage_settled = true;
        tree.lineage_integrity_lost = true;
        tree.frozen_fork_reconciliation_required = false;
        tree.pty_scan.mark_released_needs_scan_for_test();
        Self { tree, _pty: pty }
    }

    pub(super) fn register_source(&mut self) -> (TestProcess, LiveProcessIdentity) {
        let source = TestProcess::spawn();
        let live = LiveProcessIdentity::capture(source.identity)
            .unwrap()
            .unwrap();
        let _ = self.tree.events.reconcile_inventory([live]);
        assert!(self.tree.events.requires_reconciliation());
        (source, live)
    }

    pub(super) fn into_prepared_freezing_parts(self) -> (FreezingProcessTree, PtyPair) {
        let mut freezing = self.tree.begin_force();
        // This synthetic fixture has no capability scanner to restore coverage.
        freezing.tree.discovery_epoch = DiscoveryScanEpoch::Covered;
        (freezing, self._pty)
    }
}

fn reconcile_root_exit_publications(tree: &mut CapturedProcessTree) {
    let empty = std::iter::empty::<LiveProcessIdentity>;
    assert!(!tree.events.reconcile_inventory(empty()));
    assert!(tree.events.requires_reconciliation());
    assert!(tree.events.reconcile_inventory(empty()));
    assert!(!tree.events.requires_reconciliation());
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TERMINAL_ROWS,
        cols: TERMINAL_COLUMNS,
        pixel_width: TERMINAL_PIXEL_SIZE,
        pixel_height: TERMINAL_PIXEL_SIZE,
    }
}
