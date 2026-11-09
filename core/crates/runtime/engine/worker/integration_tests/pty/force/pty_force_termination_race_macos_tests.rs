use crate::process::{install_test_signal_hook, kernel_process_metadata_for_test};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const RACE_FIRST_TARGET_INDEX: usize = 0;
const RACE_NO_LEAF_KILL_ATTEMPTS: usize = 0;
const RACE_EXPECTED_LEAF_KILL_ATTEMPTS: usize = 1;
const RACE_SIGNAL_ATTEMPT_INCREMENT: usize = 1;
const RACE_EXPECTED_FIRST_FENCE_COUNT: usize = 1;
const RACE_EXPECTED_SECOND_FENCE_COUNT: usize = 2;
const RACE_KILL_PENDING_CAPABILITY: &str = "worker-kill-pending-race";
const RACE_EMPTY_FREEZE_CAPABILITY: &str = "worker-empty-freeze-race";
const RACE_STATE_TIMEOUT: Duration = Duration::from_secs(5);
const RACE_STATE_POLL_INTERVAL: Duration = Duration::from_millis(10);
const RACE_RUNNING_TIMEOUT_DETAIL: &str = "continued producer did not leave SSTOP";
const RACE_REFREEZE_DETAIL: &str = "stale output fence remained valid after SIGCONT";
const RACE_LEAF_KILLED_EARLY_DETAIL: &str = "new producer consumed SIGKILL before its output fence";

#[test]
fn sigcont_after_fence_reopens_output_drain_before_kill() {
    let (_child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Reached);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    output.run_when_fence_reaches(move || {
        assert!(root.signal(libc::SIGCONT).unwrap());
        wait_until_not_stopped(root);
    });

    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_pending(poll_force(&mut force, &mut custody, &mut output));

    assert_variant_root(&custody, CustodyKind::Freezing, root);
    assert!(root.matches(), "{RACE_REFREEZE_DETAIL}");
    assert_eq!(output.begin_calls, RACE_EXPECTED_FIRST_FENCE_COUNT);
    output.set_fence(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_eq!(output.begin_calls, RACE_EXPECTED_SECOND_FENCE_COUNT);
    assert!(root.matches(), "{RACE_REFREEZE_DETAIL}");
}

#[test]
fn kill_pending_new_producer_requires_another_output_fence() {
    let fixture = Arc::new(Mutex::new(PtyLineageFixture::spawn(
        RACE_KILL_PENDING_CAPABILITY,
    )));
    let root = fixture.lock().unwrap().root_identity();
    let tree = fixture.lock().unwrap().take_tree();
    let root_exit = tree.root_exit_proof_for_test(root);
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Reached);
    let escaped = Arc::new(Mutex::new(None));
    let escaped_from_hook = Arc::clone(&escaped);
    let fixture_from_hook = Arc::clone(&fixture);
    let hook = install_test_signal_hook(libc::SIGKILL, RACE_FIRST_TARGET_INDEX, move || {
        assert!(root.signal(libc::SIGCONT).unwrap());
        let leaf = fixture_from_hook.lock().unwrap().release_and_observe_leaf();
        *escaped_from_hook.lock().unwrap() = Some(leaf);
    });
    drive_to_killing(&mut force, &mut custody, &mut output);
    drop(hook);
    custody
        .as_mut()
        .unwrap()
        .mark_root_exited_unreaped(root_exit)
        .unwrap();
    let leaf = escaped.lock().unwrap().expect(RACE_REFREEZE_DETAIL);
    let leaf_kill_attempts = Arc::new(AtomicUsize::new(RACE_NO_LEAF_KILL_ATTEMPTS));
    let observed_leaf_kill_attempts = Arc::clone(&leaf_kill_attempts);
    let leaf_kill_hook =
        install_test_signal_hook(libc::SIGKILL, RACE_FIRST_TARGET_INDEX, move || {
            observed_leaf_kill_attempts.fetch_add(RACE_SIGNAL_ATTEMPT_INCREMENT, Ordering::AcqRel);
        });

    assert_pending(poll_force(&mut force, &mut custody, &mut output));

    assert_eq!(
        leaf_kill_attempts.load(Ordering::Acquire),
        RACE_NO_LEAF_KILL_ATTEMPTS
    );
    assert!(leaf.matches(), "{RACE_LEAF_KILLED_EARLY_DETAIL}");
    drive_until(&mut force, &mut custody, &mut output, CustodyKind::Freezing);
    assert_variant_root(&custody, CustodyKind::Freezing, root);
    assert_eq!(output.begin_calls, RACE_EXPECTED_FIRST_FENCE_COUNT);
    assert_eq!(
        leaf_kill_attempts.load(Ordering::Acquire),
        RACE_NO_LEAF_KILL_ATTEMPTS
    );
    assert!(leaf.matches(), "{RACE_LEAF_KILLED_EARLY_DETAIL}");
    output.set_fence(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_variant_root(&custody, CustodyKind::Frozen, root);
    assert_eq!(output.begin_calls, RACE_EXPECTED_SECOND_FENCE_COUNT);
    assert_eq!(
        leaf_kill_attempts.load(Ordering::Acquire),
        RACE_NO_LEAF_KILL_ATTEMPTS
    );
    assert!(leaf.matches(), "{RACE_LEAF_KILLED_EARLY_DETAIL}");
    output.set_fence(FenceBehavior::Reached);
    drive_to_killing(&mut force, &mut custody, &mut output);
    assert_eq!(
        leaf_kill_attempts.load(Ordering::Acquire),
        RACE_EXPECTED_LEAF_KILL_ATTEMPTS
    );
    drop(leaf_kill_hook);
    fixture.lock().unwrap().wait_for_leaf_exit();
    fixture.lock().unwrap().reap_root();
}

#[test]
fn descendant_created_after_empty_freeze_refreezes_and_reopens_output_fence() {
    let mut fixture = PtyLineageFixture::spawn(RACE_EMPTY_FREEZE_CAPABILITY);
    let root = fixture.root_identity();
    let tree = fixture.take_tree();
    let root_exit = tree.root_exit_proof_for_test(root);
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_eq!(output.begin_calls, RACE_EXPECTED_FIRST_FENCE_COUNT);
    match custody.as_mut().unwrap() {
        PtyProcessCustody::Frozen(tree) => tree.clear_closure_for_test(),
        _ => panic!("{RACE_REFREEZE_DETAIL}"),
    }
    assert!(root.signal(libc::SIGCONT).unwrap());
    let leaf = fixture.release_and_observe_leaf();
    custody
        .as_mut()
        .unwrap()
        .mark_root_exited_unreaped(root_exit)
        .unwrap();

    assert_pending(poll_force(&mut force, &mut custody, &mut output));

    assert_variant_root(&custody, CustodyKind::Freezing, root);
    assert!(leaf.matches(), "{RACE_LEAF_KILLED_EARLY_DETAIL}");
    drive_to_frozen(&mut force, &mut custody, &mut output);
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    assert_eq!(output.begin_calls, RACE_EXPECTED_SECOND_FENCE_COUNT);
    assert!(leaf.matches(), "{RACE_LEAF_KILLED_EARLY_DETAIL}");
    output.set_fence(FenceBehavior::Reached);
    drive_to_killing(&mut force, &mut custody, &mut output);
    fixture.wait_for_leaf_exit();
    fixture.reap_root();
}

#[test]
fn empty_kernel_eof_wait_refreezes_a_changed_process_closure() {
    let (_child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::requiring_empty_closure_output_for_test(
        EmptyClosureOutputRequirement::KernelEof,
    );
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    match custody.as_mut().unwrap() {
        PtyProcessCustody::Frozen(tree) => tree.clear_closure_for_test(),
        _ => panic!("{RACE_REFREEZE_DETAIL}"),
    }

    assert_pending(poll_force(&mut force, &mut custody, &mut output));

    assert_variant_root(&custody, CustodyKind::Freezing, root);
    assert_eq!(output.eof_acquire_calls, usize::default());
}

fn wait_until_not_stopped(root: ProcessIdentity) {
    let deadline = Instant::now() + RACE_STATE_TIMEOUT;
    loop {
        if kernel_process_metadata_for_test(root.pid)
            .unwrap()
            .status
            != libc::SSTOP
        {
            return;
        }
        assert!(Instant::now() < deadline, "{RACE_RUNNING_TIMEOUT_DETAIL}");
        std::thread::sleep(RACE_STATE_POLL_INTERVAL);
    }
}
