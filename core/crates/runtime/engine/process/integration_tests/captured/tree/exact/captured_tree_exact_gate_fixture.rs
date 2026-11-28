use super::exact_gate_authority_tests::{
    EXACT_COVERAGE_REFRESH_LIMIT, pty_size, settle_exact_coverage, wait_for_stop,
};
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT};
use crate::process::inventory_snapshot::{
    InventoryEntry, TestInventoryGuard, install_repeating_test_inventory, install_test_inventory,
};
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};
use crate::process::{
    CapturedProcessTree, FreezePoll, FrozenProcessTree, LiveProcessIdentity,
    MacosProcessCapabilityIssuer, PTY_SESSION_CAPABILITY_ENV, PtySessionProof,
};
use portable_pty::{PtyPair, native_pty_system};

const COVERAGE_SETTLEMENT_PHASE_COUNT: usize = 2;
const CAPTURE_REFRESH_COUNT: usize = 1;
const POST_FORK_REFRESH_COUNT: usize = 1;
const PARTIAL_COVERAGE_REFRESH_COUNT: usize = 3;
const INVENTORY_CAPTURE_BUDGET: usize = CAPTURE_REFRESH_COUNT
    + EXACT_COVERAGE_REFRESH_LIMIT * COVERAGE_SETTLEMENT_PHASE_COUNT
    + POST_FORK_REFRESH_COUNT
    + PARTIAL_COVERAGE_REFRESH_COUNT;
const EXPECTED_PENDING_DETAIL: &str = "exact closure skipped its confirmation poll";
const EXPECTED_FROZEN_DETAIL: &str = "exact closure did not become frozen";
const TREE_ALREADY_TAKEN_DETAIL: &str = "exact gate fixture tree was already taken";
const EXPECTED_KERNEL_EOF_DETAIL: &str = "fixture did not select kernel EOF authority";

pub(super) struct ExactGateFixture {
    tree: Option<CapturedProcessTree>,
    _escaped: TestProcess,
    late: Option<TestProcess>,
    observed_inventory: Vec<InventoryEntry>,
    inventory: Option<TestInventoryGuard>,
    _pty: PtyPair,
}

impl ExactGateFixture {
    pub(super) fn spawn(capability: &str, with_late_writer: bool) -> Self {
        let pty = native_pty_system().openpty(pty_size()).unwrap();
        let issuer = MacosProcessCapabilityIssuer::create().unwrap();
        let (reader, writer) = issuer.transfer_for_test().unwrap();
        let mut root = TestProcess::spawn_with_environment_and_process_capability(
            PTY_SESSION_CAPABILITY_ENV,
            capability,
            &writer,
        );
        let escaped = TestProcess::spawn_with_environment_and_process_capability(
            PTY_SESSION_CAPABILITY_ENV,
            capability,
            &writer,
        );
        let escaped_live = LiveProcessIdentity::capture(escaped.identity)
            .unwrap()
            .unwrap();
        reader.verify_writer_holder(&escaped_live).unwrap();
        let late = with_late_writer.then(|| {
            TestProcess::spawn_with_environment_and_process_capability(
                PTY_SESSION_CAPABILITY_ENV,
                capability,
                &writer,
            )
        });
        if let Some(late) = &late {
            let live = LiveProcessIdentity::capture(late.identity)
                .unwrap()
                .unwrap();
            reader.verify_writer_holder(&live).unwrap();
        }
        drop(writer);
        let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
        let escaped_entry = entry(escaped.identity, NO_PARENT_PID, escaped.identity.pid);
        let initial_inventory = vec![root_entry, escaped_entry];
        let mut observed_inventory = vec![escaped_entry];
        if let Some(late) = &late {
            observed_inventory.push(entry(
                late.identity,
                escaped.identity.pid,
                late.identity.pid,
            ));
        }
        let mut inventory = install_repeated_inventory(initial_inventory);
        let mut proof =
            PtySessionProof::capture_bound(pty.master.as_ref(), capability.to_owned()).unwrap();
        proof.install_process_capability(reader).unwrap();
        let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
        tree.begin_frozen_discovery();
        root.terminate();
        drop(inventory);
        inventory = install_repeated_inventory(vec![escaped_entry]);
        settle_exact_coverage(&mut tree);
        tree.terminate_captured(libc::SIGSTOP).unwrap();
        wait_for_stop(&tree);
        Self {
            tree: Some(tree),
            _escaped: escaped,
            late,
            observed_inventory,
            inventory: Some(inventory),
            _pty: pty,
        }
    }

    pub(super) fn take_tree(&mut self) -> CapturedProcessTree {
        self.tree.take().expect(TREE_ALREADY_TAKEN_DETAIL)
    }

    pub(super) fn slave_path(&self) -> std::path::PathBuf {
        self._pty.master.tty_name().unwrap()
    }

    pub(super) fn take_frozen(&mut self) -> FrozenProcessTree {
        let freezing = match self.take_tree().begin_force().poll().unwrap() {
            FreezePoll::Pending(freezing) => freezing,
            FreezePoll::Frozen(_) => panic!("{EXPECTED_PENDING_DETAIL}"),
        };
        match freezing.poll().unwrap() {
            FreezePoll::Frozen(frozen) => frozen,
            FreezePoll::Pending(_) => panic!("{EXPECTED_FROZEN_DETAIL}"),
        }
    }

    pub(super) fn take_ordinary_frozen(&mut self) -> FrozenProcessTree {
        let mut frozen = self.take_frozen();
        frozen.downgrade_kill_basis_to_ordinary_for_test();
        frozen
    }

    pub(super) fn take_kernel_eof_frozen(&mut self) -> FrozenProcessTree {
        let mut tree = self.take_tree();
        tree.discovery_epoch = DiscoveryScanEpoch::Pending;
        tree.capability_scan_pending = true;
        tree.lineage_integrity_lost = false;
        tree.pty_scan = Default::default();
        tree.pty_scan.mark_released_needs_scan_for_test();
        let freezing = match tree.begin_force().poll().unwrap() {
            FreezePoll::Pending(freezing) => freezing,
            FreezePoll::Frozen(_) => panic!("{EXPECTED_KERNEL_EOF_DETAIL}"),
        };
        self.finish_kernel_eof_freeze(freezing)
    }

    fn finish_kernel_eof_freeze(
        &self,
        mut freezing: crate::process::FreezingProcessTree,
    ) -> FrozenProcessTree {
        let deadline = std::time::Instant::now() + TEST_TIMEOUT;
        loop {
            let ready = freezing
                .prepare_poll()
                .unwrap()
                .authorize_kernel_eof_for_test();
            match freezing.commit_poll(ready) {
                FreezePoll::Frozen(frozen) => {
                    let _ = frozen.validated_kernel_eof_for_test();
                    return frozen;
                }
                FreezePoll::Pending(next) => freezing = next,
            }
            assert!(
                std::time::Instant::now() < deadline,
                "{EXPECTED_KERNEL_EOF_DETAIL}: {:?}",
                freezing.state_snapshot_for_test()
            );
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    pub(super) fn install_observed_inventory(&mut self) {
        self.inventory.take();
        self.inventory = Some(install_repeated_inventory(self.observed_inventory.clone()));
    }

    pub(super) fn install_inventory_omitting(&mut self, omitted: crate::process::ProcessIdentity) {
        self.inventory.take();
        let complete = self.observed_inventory.clone();
        let incomplete = complete
            .iter()
            .copied()
            .filter(|entry| entry.identity() != omitted)
            .collect::<Vec<_>>();
        let snapshots = std::iter::repeat_n((), INVENTORY_CAPTURE_BUDGET)
            .flat_map(|_| [complete.clone(), incomplete.clone()]);
        self.inventory = Some(install_test_inventory(snapshots));
    }

    pub(super) fn late_identity(&self) -> Option<crate::process::ProcessIdentity> {
        self.late.as_ref().map(|process| process.identity)
    }
}

fn install_repeated_inventory(inventory: Vec<InventoryEntry>) -> TestInventoryGuard {
    install_repeating_test_inventory(inventory)
}
