use crate::process::install_test_live_signal_failure;
use std::panic::{AssertUnwindSafe, catch_unwind};

const FENCE_CALLBACK_PANIC_DETAIL: &str = "injected output-fence callback panic";
const SIGNAL_HOOK_PANIC_DETAIL: &str = "injected process-transition signal hook panic";
const REPORT_CALLBACK_PANIC_DETAIL: &str = "injected producer-report callback panic";
const MASTER_PROOF_DROP_PANIC_DETAIL: &str = "injected master-close proof drop panic";
const MASTER_SETTLEMENT_PANIC_DETAIL: &str = "injected master-close settlement panic";
const UNEXPECTED_MASTER_CUSTODY_DETAIL: &str = "master-close custody changed unexpectedly";
const EXPECTED_CALLBACK_PANIC_DETAIL: &str = "output-fence callback did not panic";
const EXPECTED_TRANSITION_PANIC_DETAIL: &str = "process transition hook did not panic";
const EXPECTED_REPORT_PANIC_DETAIL: &str = "producer-report callback did not panic";
#[cfg(target_os = "linux")]
const LINUX_CONTAINMENT_REQUIRED_DETAIL: &str =
    "synthetic master closure has no Linux containment authority";

#[test]
fn fence_callback_panic_preserves_handoff_custody() {
    let (_child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
    drive_to_frozen(&mut force, &mut custody, &mut output);
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    output.panic_on_next_fence_poll(FENCE_CALLBACK_PANIC_DETAIL);

    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = poll_force(&mut force, &mut custody, &mut output);
    }));

    assert!(panic.is_err(), "{EXPECTED_CALLBACK_PANIC_DETAIL}");
    assert_variant_root(&custody, CustodyKind::Frozen, root);
    assert!(force.output_fence.is_some());
    let mut handed_off = custody.take();
    output.set_fence(FenceBehavior::Reached);
    drive_to_killing(&mut force, &mut handed_off, &mut output);
    assert_variant_root(&handed_off, CustodyKind::Killing, root);
}

#[test]
fn transition_hook_panic_preserves_handoff_custody() {
    let (_child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Pending);
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    let hook = install_test_live_signal_failure(|| panic!("{SIGNAL_HOOK_PANIC_DETAIL}"));

    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = poll_force(&mut force, &mut custody, &mut output);
    }));

    drop(hook);
    assert!(panic.is_err(), "{EXPECTED_TRANSITION_PANIC_DETAIL}");
    assert_variant_root(&custody, CustodyKind::Freezing, root);
    let mut handed_off = custody.take();
    drive_to_frozen(&mut force, &mut handed_off, &mut output);
    output.set_fence(FenceBehavior::Reached);
    drive_to_killing(&mut force, &mut handed_off, &mut output);
    assert_variant_root(&handed_off, CustodyKind::Killing, root);
}

#[test]
fn report_callback_panic_preserves_handoff_custody() {
    let (mut child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let proof = tree.root_exit_proof_for_test(root);
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Reached);
    drive_to_gone_pending(&mut child, proof, &mut force, &mut custody, &mut output);
    output.panic_on_next_report(REPORT_CALLBACK_PANIC_DETAIL);

    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = poll_force(&mut force, &mut custody, &mut output);
    }));

    assert!(panic.is_err(), "{EXPECTED_REPORT_PANIC_DETAIL}");
    assert_variant_root(&custody, CustodyKind::GonePending, root);
    let mut handed_off = custody.take();
    assert_pending(poll_force(&mut force, &mut handed_off, &mut output));
    assert_variant_root(&handed_off, CustodyKind::GoneReported, root);
}

#[test]
fn master_proof_drop_panic_preserves_close_pending_custody() {
    let (_child, root, mut custody) = master_close_pending_fixture();
    let proof = crate::process::PtyMasterAliasesClosed::scripted_with_drop_panic(
        MASTER_PROOF_DROP_PANIC_DETAIL,
    );

    let panic = catch_unwind(AssertUnwindSafe(|| {
        let _ = super::master_release::commit_master_closed(&mut custody, Some(proof));
    }));

    assert!(panic.is_err());
    assert_unretired_master_close_pending(&custody, root);
    retry_master_close(&mut custody);
}

#[test]
fn master_settlement_panic_preserves_close_pending_custody() {
    let (_child, root, mut custody) = master_close_pending_fixture();
    let Some(PtyProcessCustody::MasterClosePending(completion)) = custody.as_mut() else {
        panic!("{UNEXPECTED_MASTER_CUSTODY_DETAIL}");
    };
    completion.panic_during_master_release_settlement(MASTER_SETTLEMENT_PANIC_DETAIL);

    let panic = catch_unwind(AssertUnwindSafe(|| {
        let proof = crate::process::PtyMasterAliasesClosed::scripted();
        let _ = super::master_release::commit_master_closed(&mut custody, Some(proof));
    }));

    assert!(panic.is_err());
    assert_unretired_master_close_pending(&custody, root);
    retry_master_close(&mut custody);
}

fn master_close_pending_fixture() -> (ChildGuard, ProcessIdentity, Option<PtyProcessCustody>) {
    let (mut child, tree) = ChildGuard::spawn();
    let root = tree.root();
    let root_exit = tree.root_exit_proof_for_test(root);
    let mut custody = Some(PtyProcessCustody::tracking(tree));
    let mut force = PtyForceTermination::new();
    let mut output = ScriptedForceOutput::new(FenceBehavior::Reached);
    drive_to_gone_pending(&mut child, root_exit, &mut force, &mut custody, &mut output);
    assert_pending(poll_force(&mut force, &mut custody, &mut output));
    let state = custody.take().unwrap();
    let PtyProcessCustody::GoneReported(completion) = state else {
        panic!("{UNEXPECTED_MASTER_CUSTODY_DETAIL}");
    };
    custody = Some(PtyProcessCustody::MasterClosePending(
        completion.into_master_release_for_test(),
    ));
    (child, root, custody)
}

fn assert_unretired_master_close_pending(
    custody: &Option<PtyProcessCustody>,
    root: ProcessIdentity,
) {
    assert_variant_root(custody, CustodyKind::MasterClosePending, root);
    let Some(PtyProcessCustody::MasterClosePending(completion)) = custody.as_ref() else {
        panic!("{UNEXPECTED_MASTER_CUSTODY_DETAIL}");
    };
    assert!(!completion.session_is_retired());
}

fn retry_master_close(custody: &mut Option<PtyProcessCustody>) {
    let result = super::master_release::commit_master_closed(custody, None);
    #[cfg(target_os = "linux")]
    {
        let error = result.err().expect(LINUX_CONTAINMENT_REQUIRED_DETAIL);
        assert!(error.is::<crate::process::DestructiveSessionUnavailable>());
        assert!(!custody.as_ref().unwrap().session_is_retired());
    }
    #[cfg(not(target_os = "linux"))]
    {
        assert!(matches!(result.unwrap(), ForceTerminationPoll::Complete));
        assert!(custody.as_ref().unwrap().session_is_retired());
    }
}
