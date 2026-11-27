use crate::worker::tests::*;
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn runtime_inode_aba_between_guardian_prepare_and_worker_arm_is_rejected() {
    let fixture = Fixture::new();
    let previous = lock_recover(&fixture.state.record).clone();
    let previous_generation = fixture.state.controller_generation.load(Ordering::Acquire);
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    let preparation = wire.prepare_guardian(&offer);

    let runtime_path = fixture.state.paths.runtime_record(fixture.terminal_id());
    let before: (RuntimeRecord, crate::paths::identity::FileIdentity) = fixture
        .state
        .namespace
        .read_json_with_identity(&runtime_path)
        .unwrap();
    fixture
        .state
        .namespace
        .write_json(&runtime_path, &before.0)
        .unwrap();
    let after: (RuntimeRecord, crate::paths::identity::FileIdentity) = fixture
        .state
        .namespace
        .read_json_with_identity(&runtime_path)
        .unwrap();
    assert_eq!(after.0, before.0);
    assert!(!after.1.same_object(before.1));

    wire.peer
        .as_mut()
        .unwrap()
        .write(&crate::pty_custody::CustodyHostAck::prepared_guarded(
            &offer,
            preparation,
        ))
        .await
        .unwrap();
    await_adoption_failure(adoption).await;
    let commit: anyhow::Result<crate::pty_custody::CustodyWorkerCommit> =
        wire.peer.as_mut().unwrap().read().await;
    assert!(commit.is_err());

    let custody: crate::pty_custody::CustodyDurableRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.pty_custody(fixture.terminal_id()))
        .unwrap();
    assert_eq!(
        custody
            .prepared_guardian_candidate()
            .and_then(|candidate| candidate.guardian),
        Some(preparation.binding())
    );
    assert_eq!(*lock_recover(&fixture.state.record), previous);
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        previous_generation
    );
    assert_eq!(
        fixture
            .state
            .controller_persisted_generation
            .load(Ordering::Acquire),
        previous_generation
    );
}
