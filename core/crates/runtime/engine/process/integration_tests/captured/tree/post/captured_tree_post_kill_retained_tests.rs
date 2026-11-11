use super::super::super::termination::{ExactKillAuthority, RetainedExclusion};
use super::*;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess};
use crate::process::{LiveProcessIdentity, MacosProcessCapabilityIssuer, PtySessionProof};
use portable_pty::{PtyPair, PtySize, native_pty_system};

const TEST_CAPABILITY: &str = "post-kill-retained-exclusion";
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_PIXEL_SIZE: u16 = 0;
const EXACT_EMPTY_DETAIL: &str = "retained exact exclusion did not settle the empty closure";
const REVOKED_DETAIL: &str = "revoked retained exclusion authorized an empty closure";

#[test]
fn pending_exact_positive_settles_empty_with_capability_debt() {
    let mut fixture = RetainedFixture::new(ExactKillAuthority::ExactPositive);
    fixture.root.terminate();

    assert_observation(&mut fixture, PostKillObservation::Pending);
    assert!(fixture.tree.root_exit_is_proven(), "{EXACT_EMPTY_DETAIL}");
    assert!(fixture.tree.capability_scan_pending);
    assert_observation(&mut fixture, PostKillObservation::ExactEmpty);
}

#[test]
fn transient_publication_lineage_debt_stays_exact_then_recovers() {
    let mut fixture = RetainedFixture::new(ExactKillAuthority::ExactPositive);
    fixture.root.terminate();
    assert_observation(&mut fixture, PostKillObservation::Pending);
    fixture
        .tree
        .force_next_frozen_fork_publication_gap_for_test();

    assert_observation(&mut fixture, PostKillObservation::Pending);
    assert!(fixture.tree.lineage_integrity_lost);
    assert!(!fixture.tree.events.has_unresolved_fork());
    assert!(!fixture.tree.frozen_fork_reconciliation_required);
    assert_eq!(fixture.retained_exclusion(), RetainedExclusion::Pending);
    assert_observation(&mut fixture, PostKillObservation::ExactEmpty);
}

#[test]
fn closure_version_change_refreezes() {
    let mut fixture = RetainedFixture::new(ExactKillAuthority::ExactPositive);
    fixture.tree.closure_version = fixture.tree.closure_version.next().unwrap();

    assert_observation(&mut fixture, PostKillObservation::Refreeze);
    assert_eq!(fixture.retained_exclusion(), RetainedExclusion::Revoked);
}

#[test]
fn pid_generation_replacement_refreezes() {
    let mut fixture = RetainedFixture::new(ExactKillAuthority::ExactPositive);
    let replacement = TestProcess::spawn();
    let replacement_generation = live_identity(&replacement).unique_id();
    let replaced = fixture.killed.with_generation(replacement_generation);
    fixture.tree.captured.clear();
    fixture
        .tree
        .captured
        .insert(replaced, fixture.root.identity);
    fixture.tree.ownership = super::super::super::ownership::SessionOwnership::Retired;

    assert_observation(&mut fixture, PostKillObservation::Refreeze);
    assert_eq!(fixture.retained_exclusion(), RetainedExclusion::Revoked);
}

#[test]
fn late_target_waits_for_publication_then_refreezes() {
    let mut fixture = RetainedFixture::new(ExactKillAuthority::ExactPositive);
    let late = TestProcess::spawn();
    fixture.tree.bind_repeating_process_inventory_for_test(&[
        (
            fixture.root.identity,
            NO_PARENT_PID,
            fixture.root.identity.pid,
        ),
        (
            late.identity,
            fixture.root.identity.pid,
            fixture.root.identity.pid,
        ),
    ]);

    assert_observation(&mut fixture, PostKillObservation::Pending);
    assert_observation(&mut fixture, PostKillObservation::Refreeze);
    assert_eq!(fixture.retained_exclusion(), RetainedExclusion::Revoked);
}

#[test]
fn incremental_authority_rebuilds_global_exclusion() {
    let mut fixture = RetainedFixture::new(ExactKillAuthority::IncrementalExactPositive);
    fixture.root.terminate();

    assert_observation(&mut fixture, PostKillObservation::Pending);
    let observed = fixture.wait.observe(&mut fixture.tree).unwrap();
    assert!(
        !matches!(observed, PostKillObservation::ExactEmpty),
        "{REVOKED_DETAIL}"
    );
    assert_eq!(fixture.retained_exclusion(), RetainedExclusion::Revoked);
}

#[test]
fn event_gate_loss_irrevocably_revokes_retained_exclusion() {
    let mut fixture = RetainedFixture::new(ExactKillAuthority::ExactPositive);
    fixture.root.terminate();
    assert_observation(&mut fixture, PostKillObservation::Pending);
    fixture.tree.events.disable_gate_for_test();

    let observed = fixture.wait.observe(&mut fixture.tree).unwrap();
    assert!(
        !matches!(observed, PostKillObservation::ExactEmpty),
        "{REVOKED_DETAIL}"
    );
    assert_eq!(fixture.retained_exclusion(), RetainedExclusion::Revoked);
}

pub(super) struct RetainedFixture {
    pub(super) root: TestProcess,
    pub(super) killed: LiveProcessIdentity,
    pub(super) tree: CapturedProcessTree,
    pub(super) wait: PostKillExactClosure,
    _writer: crate::process::MacosProcessCapabilityChildWriter,
    _pty: PtyPair,
}

impl RetainedFixture {
    pub(super) fn new(authority: ExactKillAuthority) -> Self {
        let pty = native_pty_system().openpty(pty_size()).unwrap();
        let issuer = MacosProcessCapabilityIssuer::create().unwrap();
        let (reader, writer) = issuer.transfer_for_test().unwrap();
        let root = TestProcess::spawn();
        let killed = live_identity(&root);
        let mut proof =
            PtySessionProof::capture_bound(pty.master.as_ref(), TEST_CAPABILITY.to_owned())
                .unwrap();
        proof.install_process_capability(reader).unwrap();
        let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
        tree.begin_frozen_discovery();
        tree.bind_repeating_process_inventory_for_test(&[]);
        let kill = super::super::super::termination::ExactClosureKill::for_test(
            vec![(killed, root.identity)],
            tree.closure_version.clone(),
            authority,
        );
        let wait = PostKillExactClosure::exact(kill);
        Self {
            root,
            killed,
            tree,
            wait,
            _writer: writer,
            _pty: pty,
        }
    }

    pub(super) fn retained_exclusion(&self) -> RetainedExclusion {
        let PostKillPhase::Exact {
            retained_exclusion, ..
        } = &self.wait.phase
        else {
            panic!("{REVOKED_DETAIL}");
        };
        *retained_exclusion
    }
}

pub(super) fn assert_observation(fixture: &mut RetainedFixture, expected: PostKillObservation) {
    let observed = fixture.wait.observe(&mut fixture.tree).unwrap();
    assert_eq!(
        observed, expected,
        "{EXACT_EMPTY_DETAIL}: expected={expected:?} observed={observed:?}"
    );
}

fn live_identity(process: &TestProcess) -> LiveProcessIdentity {
    LiveProcessIdentity::capture(process.identity)
        .unwrap()
        .unwrap()
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TERMINAL_ROWS,
        cols: TERMINAL_COLUMNS,
        pixel_width: TERMINAL_PIXEL_SIZE,
        pixel_height: TERMINAL_PIXEL_SIZE,
    }
}
