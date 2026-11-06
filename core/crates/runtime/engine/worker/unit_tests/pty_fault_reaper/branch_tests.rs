fn branch_reaper() -> FaultReap {
    fault_reap_construction::passive_fault_reap_for_test()
}

#[test]
fn semantic_progress_takes_priority_over_scan_continuation() {
    let root = ProcessIdentity::capture(std::process::id()).unwrap();
    let mut tree = CapturedProcessTree::capture(root).unwrap();
    tree.record_capability_scan_activity_for_test(
        crate::process::CapabilityScanActivity::ContinueRound,
    );
    let mut reaper = branch_reaper();
    reaper.custody_mut().processes = Some(PtyProcessCustody::tracking(tree));
    let previous = reaper.progress();
    reaper.root_gone_without_wait = true;

    assert_eq!(
        reaper.take_progress_after(previous),
        schedule::FaultReapActivity::SemanticProgress
    );
    let settled = reaper.progress();
    assert_eq!(
        reaper.take_progress_after(settled),
        schedule::FaultReapActivity::Idle
    );
}

#[test]
fn scan_continuation_is_not_reported_as_semantic_progress() {
    let root = ProcessIdentity::capture(std::process::id()).unwrap();
    let mut tree = CapturedProcessTree::capture(root).unwrap();
    tree.record_capability_scan_activity_for_test(
        crate::process::CapabilityScanActivity::ContinueRound,
    );
    let mut reaper = branch_reaper();
    reaper.custody_mut().processes = Some(PtyProcessCustody::tracking(tree));
    let previous = reaper.progress();

    assert_eq!(
        reaper.take_progress_after(previous),
        schedule::FaultReapActivity::ScanContinuation
    );
}

#[test]
fn missing_wait_authority_does_not_publish_false_reap_completion() {
    let root = ProcessIdentity::capture(std::process::id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let proof = tree.root_exit_proof_for_test(root);
    let mut reaper = branch_reaper();

    assert!(!reaper.finish_reaped(proof));
    assert!(!reaper.completion().is_published());
    assert!(reaper.custody().authority.is_none());
}

#[test]
fn externally_reaped_exact_proof_retains_lost_wait_custody() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let proof = tree.root_exit_proof_for_test(root);
    let child = Box::new(child) as crate::process::OwnedPtyChild;
    let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
        .unwrap()
        .bind(child);
    assert!(root.signal(libc::SIGKILL).unwrap());
    assert!(wait_direct_child(root.pid).status.is_some());
    let mut reaper = branch_reaper();
    reaper.custody_mut().authority = Some(authority);
    assert!(reaper.has_unsettled_custody());
    reaper.custody_mut().processes = Some(PtyProcessCustody::tracking(tree));

    assert!(!reaper.finish_reaped(proof));

    assert!(reaper.custody().authority.is_none());
    assert!(reaper.lost_wait_custody.is_some());
    assert!(!reaper.completion().is_published());
    assert!(reaper.has_unsettled_custody());
}

#[test]
fn successful_bookkeeping_does_not_replace_a_retained_fault() {
    const RETAINED_FAULT: &str = "retained fault";
    let mut reaper = branch_reaper();
    reaper.remember(Err(anyhow::anyhow!(RETAINED_FAULT)));

    reaper.remember(Ok(()));

    assert_eq!(reaper.failure.as_ref().unwrap().to_string(), RETAINED_FAULT);
}

#[test]
fn authority_loss_without_live_authority_does_not_create_false_custody() {
    let error = anyhow::Error::new(std::io::Error::from_raw_os_error(libc::ECHILD));
    let loss = WaitAuthorityLoss::from_error(&error).unwrap();
    let mut reaper = branch_reaper();

    reaper.lose_wait_authority(loss);

    assert!(reaper.custody().authority.is_none());
    assert!(reaper.lost_wait_custody.is_none());
    assert!(!reaper.has_unsettled_custody());
}
