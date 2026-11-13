use super::*;
use crate::process::{
    CapturedProcessTree, FreezePoll, FrozenProcessTree, KillCommitPoll, KillPendingProcessTree,
    KillPoll, LostRootWaitAuthority, ProcessIdentity, RootGoneWithoutWait, RootWaitAuthority,
    WaitAuthorityLoss, wait_direct_child,
};
use crate::worker::pty_process_custody::{PtyProcessCustodyKind, PtyProcessCustodySnapshot};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const TRANSITION_TIMEOUT: Duration = Duration::from_secs(5);
const TRANSITION_POLL_INTERVAL: Duration = Duration::from_millis(10);
const TRANSITION_TIMEOUT_DETAIL: &str = "process custody did not reach its requested state";
const UNEXPECTED_REFREEZE_DETAIL: &str = "stable test closure unexpectedly required refreeze";

#[derive(Clone, Copy)]
enum Stage {
    Tracking,
    Freezing,
    Frozen,
    Killing,
    GonePending,
    GoneReported,
}

include!("../../../test_support/integration/pty/pty_process_custody_root_gone_fixture.rs");

#[test]
fn exact_gone_proof_is_accepted_by_every_typed_custody_state() {
    for stage in [
        Stage::Tracking,
        Stage::Freezing,
        Stage::Frozen,
        Stage::Killing,
        Stage::GonePending,
        Stage::GoneReported,
    ] {
        assert_stage_accepts(stage);
    }
}

fn assert_stage_accepts(stage: Stage) {
    let mut fixture = Fixture::spawn();
    let (mut custody, requires_signal) = custody_at(stage, &mut fixture);
    custody = match custody {
        PtyProcessCustody::GoneReported(reported) => {
            PtyProcessCustody::GoneReported(reported.retire_direct_for_test())
        }
        custody => custody,
    };
    if requires_signal {
        assert!(fixture.root.signal(libc::SIGKILL).unwrap());
    }
    let (lost, proof) = fixture.reap_and_lose();
    lost.validate(&proof).unwrap();

    custody.mark_root_gone_without_wait(proof).unwrap();

    assert_eq!(
        custody.snapshot_for_test(),
        expected_snapshot(stage, fixture.root)
    );
}

fn custody_at(stage: Stage, fixture: &mut Fixture) -> (PtyProcessCustody, bool) {
    let tree = fixture.take_tree();
    match stage {
        Stage::Tracking => (PtyProcessCustody::Tracking(tree), true),
        Stage::Freezing => (PtyProcessCustody::Freezing(tree.begin_force()), true),
        Stage::Frozen => (PtyProcessCustody::Frozen(drive_frozen(tree)), true),
        Stage::Killing => (
            PtyProcessCustody::Killing(begin_killing(drive_frozen(tree))),
            false,
        ),
        Stage::GonePending | Stage::GoneReported => gone_custody(stage, fixture, tree),
    }
}

fn gone_custody(
    stage: Stage,
    fixture: &Fixture,
    tree: CapturedProcessTree,
) -> (PtyProcessCustody, bool) {
    let mut killing = begin_killing(drive_frozen(tree));
    killing
        .mark_root_exited_unreaped(fixture.observe_unreaped())
        .unwrap();
    let gone = drive_gone(killing);
    if matches!(stage, Stage::GonePending) {
        return (PtyProcessCustody::GonePending(gone), false);
    }
    let (reported, ()) = gone.report(|_| Ok(())).unwrap();
    (PtyProcessCustody::GoneReported(reported), false)
}

fn drive_frozen(tree: CapturedProcessTree) -> FrozenProcessTree {
    let deadline = Instant::now() + TRANSITION_TIMEOUT;
    let mut freezing = tree.begin_force();
    loop {
        freezing = match freezing.poll().unwrap() {
            FreezePoll::Pending(pending) => pending,
            FreezePoll::Frozen(frozen) => return frozen,
        };
        assert!(Instant::now() < deadline, "{TRANSITION_TIMEOUT_DETAIL}");
    }
}

fn begin_killing(frozen: FrozenProcessTree) -> KillPendingProcessTree {
    let version = frozen.closure_version();
    match frozen.kill_after_output_fence(version).unwrap() {
        KillCommitPoll::Pending(pending) => pending,
        KillCommitPoll::Refreeze(_) => panic!("{UNEXPECTED_REFREEZE_DETAIL}"),
    }
}

fn drive_gone(mut killing: KillPendingProcessTree) -> crate::process::TrackedProducersGone {
    let deadline = Instant::now() + TRANSITION_TIMEOUT;
    loop {
        killing = match killing.poll().unwrap() {
            KillPoll::Pending(pending) => pending,
            KillPoll::Gone(gone) => return gone,
            KillPoll::Refreeze(_) => panic!("{UNEXPECTED_REFREEZE_DETAIL}"),
        };
        assert!(Instant::now() < deadline, "{TRANSITION_TIMEOUT_DETAIL}");
        std::thread::sleep(TRANSITION_POLL_INTERVAL);
    }
}

fn expected_snapshot(stage: Stage, root: ProcessIdentity) -> PtyProcessCustodySnapshot {
    let kind = match stage {
        Stage::Tracking => PtyProcessCustodyKind::Tracking,
        Stage::Freezing => PtyProcessCustodyKind::Freezing,
        Stage::Frozen => PtyProcessCustodyKind::Frozen,
        Stage::Killing => PtyProcessCustodyKind::Killing,
        Stage::GonePending => PtyProcessCustodyKind::GonePending,
        Stage::GoneReported => PtyProcessCustodyKind::GoneReported,
    };
    PtyProcessCustodySnapshot { kind, root }
}
