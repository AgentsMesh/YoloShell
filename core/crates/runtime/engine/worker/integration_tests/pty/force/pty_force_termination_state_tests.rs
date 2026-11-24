#[cfg(target_os = "macos")]
use crate::process::{PtyLineageFixture, install_test_match_failure};
use crate::process::{
    install_test_live_signal_failure as install_unavailable_signal_failure,
    install_test_signal_failure,
};
use std::cell::Cell;
use std::rc::Rc;
const FREEZE_FAILURE_DETAIL: &str = "injected worker force freeze failure";
const FENCE_BEGIN_FAILURE_DETAIL: &str = "injected worker output-fence begin failure";
const FENCE_POLL_FAILURE_DETAIL: &str = "injected worker output-fence poll failure";
const KILL_FAILURE_DETAIL: &str = "injected worker force kill failure";
#[cfg(target_os = "macos")]
const KILL_POLL_FAILURE_DETAIL: &str = "injected worker kill-pending poll failure";
#[cfg(target_os = "macos")]
const KILL_POLL_CAPABILITY: &str = "worker-kill-poll-fallback";
const FORBIDDEN_SIGNAL_DETAIL: &str = "unavailable process signal attempted";
const FIRST_TARGET_INDEX: usize = 0;
const NO_SIGNAL_CALLS: usize = 0;
enum UnavailableInitialCustody {
    Tracking,
    Freezing,
}

#[test]
fn freeze_error_preserves_freezing_authority() {
    let (_child, mut tree) = ChildGuard::spawn();
    let root = tree.root();
    tree.fail_next_refresh_for_test(FREEZE_FAILURE_DETAIL);
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);

    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    let error = expect_error(poll_force(&mut force, &mut custody, &mut output));

    assert_error_contains(&error, FREEZE_FAILURE_DETAIL);
    assert_variant_root(&custody, CustodyKind::Freezing, root);
}

#[test]
fn fence_begin_error_preserves_frozen_authority() {
    let (_child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output =
        ScriptedForceOutput::new(FenceBehavior::BeginError(FENCE_BEGIN_FAILURE_DETAIL));
    drive_to_frozen(&mut force, &mut custody, &mut output);

    let error = expect_error(poll_force(&mut force, &mut custody, &mut output));

    assert_error_contains(&error, FENCE_BEGIN_FAILURE_DETAIL);
    assert_variant_root(&custody, CustodyKind::Frozen, root);
}

#[test]
fn fence_poll_error_preserves_frozen_authority() {
    let (_child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::PollError(FENCE_POLL_FAILURE_DETAIL));
    drive_to_frozen(&mut force, &mut custody, &mut output);
    assert_pending(poll_force(&mut force, &mut custody, &mut output));

    let error = expect_error(poll_force(&mut force, &mut custody, &mut output));

    assert_error_contains(&error, FENCE_POLL_FAILURE_DETAIL);
    assert_variant_root(&custody, CustodyKind::Frozen, root);
}

#[test]
fn pending_fence_cannot_kill_and_kill_error_preserves_frozen() {
    let (_child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    let _failure =
        install_test_signal_failure(libc::SIGKILL, FIRST_TARGET_INDEX, KILL_FAILURE_DETAIL);

    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_variant_root(&custody, CustodyKind::Frozen, root);
    assert!(root.matches());
    output.set_fence(FenceBehavior::Reached);
    let error = expect_error(poll_force(&mut force, &mut custody, &mut output));

    assert_error_contains(&error, KILL_FAILURE_DETAIL);
    assert_variant_root(&custody, CustodyKind::Frozen, root);
}

#[test]
fn fence_creation_is_progress_without_a_custody_phase_change() {
    let (_child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut output);

    assert!(!force.take_progress());
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_variant_root(&custody, CustodyKind::Frozen, root);
    assert!(force.take_progress());
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_variant_root(&custody, CustodyKind::Frozen, root);
    assert!(!force.take_progress());
}

#[test]
fn fence_internal_transition_is_progress_without_a_custody_phase_change() {
    let (_child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut scripted = ScriptedForceOutput::new(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut scripted);
    let observed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::default());
    let mut output = crate::worker::pty_output_progress::ExclusivePtyOutput::observed(observed);

    assert_pending(force.poll(&mut custody, &mut ForceOutput::Exclusive(&mut output)));
    assert!(force.take_progress());
    assert_pending(force.poll(&mut custody, &mut ForceOutput::Exclusive(&mut output)));
    assert_variant_root(&custody, CustodyKind::Frozen, root);
    assert!(force.take_progress());
    assert!(!force.take_progress());
}

#[cfg(target_os = "macos")]
#[test]
fn kill_poll_error_preserves_authority_and_reaches_gone_pending() {
    let mut child = PtyLineageFixture::spawn(KILL_POLL_CAPABILITY);
    let tree = child.take_tree();
    let root = tree.root();
    let proof = tree.root_exit_proof_for_test(root);
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Reached);
    drive_to_killing(&mut force, &mut custody, &mut output);
    {
        let _failure = install_test_match_failure(KILL_POLL_FAILURE_DETAIL);
        let error = expect_error(poll_force(&mut force, &mut custody, &mut output));

        assert_error_contains(&error, KILL_POLL_FAILURE_DETAIL);
        assert_variant_root(&custody, CustodyKind::Killing, root);
    }
    custody
        .as_mut()
        .unwrap()
        .mark_root_exited_unreaped(proof)
        .unwrap();
    child.reap_root();
    output.observe_kernel_eof();
    drive_until(
        &mut force,
        &mut custody,
        &mut output,
        CustodyKind::GonePending,
    );
    assert_variant_root(&custody, CustodyKind::GonePending, root);
}

#[test]
fn unavailable_authority_converts_unstarted_custody_to_survivor_without_signal() {
    for initial in [
        UnavailableInitialCustody::Tracking,
        UnavailableInitialCustody::Freezing,
    ] {
        let (_child, mut tree) = ChildGuard::spawn();
        let root = tree.root();
        tree.deny_destructive_for_test();
        let state = match initial {
            UnavailableInitialCustody::Tracking => PtyProcessCustody::tracking(tree),
            UnavailableInitialCustody::Freezing => PtyProcessCustody::Freezing(tree.begin_force()),
        };
        let mut custody = Some(state);
        let mut force = PtyForceTermination::new();
        let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
        let signal_calls = Rc::new(Cell::new(NO_SIGNAL_CALLS));
        let observed = Rc::clone(&signal_calls);
        let _signal_failure = install_unavailable_signal_failure(move || {
            observed.set(observed.get() + OPERATION_CALL_INCREMENT);
            Some(FORBIDDEN_SIGNAL_DETAIL.to_owned())
        });

        assert!(matches!(
            poll_force(&mut force, &mut custody, &mut output),
            Ok(ForceTerminationPoll::Unsupported(_))
        ));
        assert_variant_root(&custody, CustodyKind::Survivor, root);
        assert_eq!(signal_calls.get(), NO_SIGNAL_CALLS);
        assert!(root.matches());
    }
}
