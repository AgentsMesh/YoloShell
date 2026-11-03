const ROOT_EXIT_PROOF_TIMEOUT: Duration = Duration::from_secs(1);

#[test]
fn preexisting_root_exit_proof_is_reconciled_into_the_tree() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let child = Box::new(child) as crate::process::OwnedPtyChild;
    let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
        .unwrap()
        .bind(child);
    root.signal(libc::SIGKILL).unwrap();
    let deadline = Instant::now() + ROOT_EXIT_PROOF_TIMEOUT;
    let proof = loop {
        if let Some(proof) = authority.poll_exited_unreaped().unwrap() {
            break proof;
        }
        assert!(
            Instant::now() < deadline,
            "root exit proof was not observed"
        );
        std::thread::yield_now();
    };
    let (completion, _receiver) = FaultReapCompletion::owner_channel();
    let mut reaper = active_reaper(authority, tree, completion);
    reaper.custody_mut().root_exit_proof = Some(proof);

    assert_eq!(
        tree_observation(&mut reaper),
        crate::process::DescendantObservation::Settled
    );
    assert!(
        !reaper
            .custody_mut()
            .processes
            .as_mut()
            .unwrap()
            .tracking_mut()
            .root_exit_is_proven_for_test()
    );
    live_authority::observe_root_exit(&mut reaper);
    assert!(
        reaper
            .custody_mut()
            .processes
            .as_mut()
            .unwrap()
            .tracking_mut()
            .root_exit_is_proven_for_test()
    );
    assert_eq!(
        tree_observation(&mut reaper),
        crate::process::DescendantObservation::Settled
    );

    assert!(
        reaper
            .custody()
            .authority
            .as_ref()
            .is_some_and(RootWaitAuthority::is_waited)
    );
}

fn tree_observation(reaper: &mut FaultReap) -> crate::process::DescendantObservation {
    reaper
        .custody_mut()
        .processes
        .as_mut()
        .unwrap()
        .tracking_mut()
        .descendant_observation_for_test()
}
