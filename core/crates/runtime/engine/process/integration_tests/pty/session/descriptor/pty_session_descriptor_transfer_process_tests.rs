use super::super::super::macos_liveness::{MacosCapabilityLivenessState, MacosGlobalScanMode};
use super::super::super::macos_released::candidate::{
    GlobalCandidateContext, evaluate_test_candidate,
};
use super::super::super::{PtyCapabilityScanState, PtySessionProof};
use super::*;
use crate::process::CapturedProcessTree;
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT};
use crate::process::inventory_snapshot::install_test_inventory;
use crate::process::inventory_stable::STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE;
use crate::process::inventory_test_support::entry;

#[path = "pty_session_descriptor_transfer_fixture_macos.rs"]
mod fixture;
use fixture::DescriptorTransferFixture;
use std::time::Instant;

const INITIAL_TREE_STABLE_CAPTURES: usize = 2;
const TRANSFER_CAPTURE_TIMEOUT_DETAIL: &str = "transferred writer was not captured";
const RECEIVER_SCAN_TIMEOUT_DETAIL: &str = "live descriptor receiver was not observed";

#[test]
fn boundary_scm_transfer_rechecks_receiver_after_root_disappears() {
    let mut fixture = DescriptorTransferFixture::launch();
    let receiver = fixture.receiver_candidate();
    let root = fixture.root_candidate();
    let current = candidates([receiver, root]);
    let mut released = ReleasedScanState::default();
    let first = advance_scan(
        current,
        &released,
        ONE_CANDIDATE_PER_BATCH,
        |candidate, budget| {
            evaluate_test_candidate(
                fixture.proof(),
                candidate,
                budget,
                GlobalCandidateContext::ReleasedTty,
                MacosGlobalScanMode::TransferableWriterPresent,
            )
        },
    )
    .unwrap();
    assert_eq!(first.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(
        first.candidates.observed,
        HashSet::from([root.live.lineage_id()])
    );
    commit(&mut released, first);

    fixture.transfer_and_exit_root();
    fixture.assert_writer_present();
    let receiver_inventory = fixture.receiver_inventory();
    let mut state = PtyCapabilityScanState {
        released,
        liveness: MacosCapabilityLivenessState::WriterPresent,
    };
    let transferred = scan_live_receiver(&fixture, &receiver_inventory, &state);
    assert_eq!(transferred.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(transferred.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(
        transferred.candidates.observed,
        HashSet::from([fixture.receiver_lineage()])
    );
    assert_eq!(
        transferred.candidates.stable,
        HashSet::from([fixture.receiver_lineage()])
    );
    state.commit(transferred.advance);
    assert!(state.released.matched.contains_key(&receiver.live));

    let retained = scan_live_receiver(&fixture, &receiver_inventory, &state);
    assert_eq!(retained.coverage_status, CapabilityScanStatus::Complete);
    assert_eq!(retained.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(
        retained.candidates.observed,
        HashSet::from([fixture.receiver_lineage()])
    );
}

#[test]
fn pre_root_unrelated_holder_is_an_exact_owned_target() {
    let mut fixture = DescriptorTransferFixture::launch();
    let receiver = fixture.receiver_candidate();
    fixture.transfer_and_exit_root();
    fixture.assert_writer_present();
    let receiver_inventory = fixture.receiver_inventory();
    let mut state = PtyCapabilityScanState {
        released: ReleasedScanState::default(),
        liveness: MacosCapabilityLivenessState::WriterPresent,
    };
    let held = scan_live_receiver(&fixture, &receiver_inventory, &state);
    assert_eq!(held.candidates.status, CapabilityScanStatus::Pending);
    assert_eq!(
        held.candidates.observed,
        HashSet::from([receiver.live.lineage_id()])
    );
    assert_eq!(
        held.candidates.stable,
        HashSet::from([receiver.live.lineage_id()])
    );
    state.commit(held.advance);
    assert!(state.released.matched.contains_key(&receiver.live));

    fixture.release_receiver();
    fixture.wait_until_released();
    let empty = inventory(Vec::new(), HashSet::new());
    let eof_scan = fixture.scan(&empty, &state);
    assert_eq!(eof_scan.candidates.status, CapabilityScanStatus::Pending);
    state.commit(eof_scan.advance);
    let settled = fixture.scan(&empty, &state);
    assert_eq!(settled.candidates.status, CapabilityScanStatus::Complete);
}

#[test]
fn destructive_closure_captures_the_pre_root_transferred_writer() {
    let mut fixture = DescriptorTransferFixture::launch();
    let root =
        crate::process::ProcessIdentity::capture(fixture.root_candidate().live.pid()).unwrap();
    let root_entry = entry(root, u32::default(), root.pid);
    let initial = install_test_inventory(std::iter::repeat_n(
        vec![root_entry],
        STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE * INITIAL_TREE_STABLE_CAPTURES,
    ));
    let mut tree = CapturedProcessTree::capture_pty(root, fixture.proof().clone()).unwrap();
    drop(initial);
    let root_exit = tree.root_exit_proof_for_test(root);

    fixture.transfer_and_exit_root();
    let receiver = fixture.receiver_candidate();
    tree.mark_root_exited_unreaped(root_exit).unwrap();
    let receiver_durable = crate::process::ProcessIdentity::capture(receiver.live.pid()).unwrap();
    let receiver_entry = entry(receiver_durable, u32::default(), receiver.live.pid());
    refresh_until_captured(&mut tree, receiver_entry, receiver_durable);

    assert!(tree.identities().contains(&receiver_durable));
    tree.authorize_destructive_for_test();
    assert!(
        tree.terminate_captured(libc::SIGKILL)
            .unwrap()
            .contains(&receiver_durable)
    );
}

fn refresh_until_captured(
    tree: &mut CapturedProcessTree,
    receiver: crate::process::inventory_snapshot::InventoryEntry,
    durable: crate::process::ProcessIdentity,
) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while !tree.identities().contains(&durable) {
        let inventory = install_test_inventory(std::iter::repeat_n(
            vec![receiver],
            STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE,
        ));
        tree.refresh().unwrap();
        drop(inventory);
        assert!(
            Instant::now() < deadline,
            "{TRANSFER_CAPTURE_TIMEOUT_DETAIL}"
        );
        std::thread::sleep(POLL_INTERVAL);
    }
}

fn scan_live_receiver(
    fixture: &DescriptorTransferFixture,
    inventory: &StableInventory,
    state: &PtyCapabilityScanState,
) -> CapabilityScanResult {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        let result = fixture.scan(inventory, state);
        if result
            .candidates
            .observed
            .contains(&fixture.receiver_lineage())
        {
            return result;
        }
        assert!(Instant::now() < deadline, "{RECEIVER_SCAN_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[test]
fn descriptor_transfer_helper() {
    fixture::run_helper();
}

fn proof_with_reader(
    reader: std::sync::Arc<crate::process::MacosProcessCapabilityManagerReader>,
) -> PtySessionProof {
    let mut proof = PtySessionProof::discovery(fixture::TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(reader).unwrap();
    proof
}
