#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
enum CommittedAdoptionFailure {
    ReadyAcknowledgementLost,
    ReleaseAcknowledged,
    ReleaseAcknowledgementLost,
    CancelledBeforePromotion,
    CancelledAfterPromotion,
}

#[cfg(target_os = "macos")]
async fn fail_committed_adoption(
    fixture: &Fixture,
    boundary: CommittedAdoptionFailure,
) -> SuccessorAdoptionWire {
    match boundary {
        CommittedAdoptionFailure::ReleaseAcknowledged
        | CommittedAdoptionFailure::ReleaseAcknowledgementLost => {
            fixture
                .state
                .control_test_hooks
                .fail_next_controller_persist();
        }
        CommittedAdoptionFailure::CancelledBeforePromotion => {
            fixture
                .state
                .control_test_hooks
                .custody_before_promotion
                .arm();
        }
        CommittedAdoptionFailure::CancelledAfterPromotion => {
            fixture
                .state
                .control_test_hooks
                .custody_before_activation
                .arm();
        }
        CommittedAdoptionFailure::ReadyAcknowledgementLost => {}
    }
    let mut wire = SuccessorAdoptionWire::install(fixture);
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    wire.receive_commit(&offer).await;
    if !matches!(boundary, CommittedAdoptionFailure::ReadyAcknowledgementLost) {
        wire.send_ready(&offer).await;
    }
    match boundary {
        CommittedAdoptionFailure::CancelledBeforePromotion
        | CommittedAdoptionFailure::CancelledAfterPromotion => {
            let hook = match boundary {
                CommittedAdoptionFailure::CancelledBeforePromotion => {
                    &fixture.state.control_test_hooks.custody_before_promotion
                }
                _ => &fixture.state.control_test_hooks.custody_before_activation,
            };
            hook.wait_until_reached().await;
            adoption.abort();
            assert!(adoption.await.unwrap_err().is_cancelled());
            return wire;
        }
        CommittedAdoptionFailure::ReleaseAcknowledged => {
            acknowledge_exact_worker_release(wire.peer.as_mut().unwrap(), &offer).await;
        }
        CommittedAdoptionFailure::ReleaseAcknowledgementLost => {
            let disposition: crate::pty_custody::CustodyWorkerReadyDisposition =
                wire.peer.as_mut().unwrap().read().await.unwrap();
            assert!(matches!(disposition,
                crate::pty_custody::CustodyWorkerReadyDisposition::Release { control }
                    if control.matches_release(&offer)));
        }
        CommittedAdoptionFailure::ReadyAcknowledgementLost => {}
    }
    drop(wire.take_peer());
    await_adoption_failure(adoption).await;
    wire
}

#[cfg(target_os = "macos")]
fn committed_custody_artifact(
    fixture: &Fixture,
) -> (
    crate::pty_custody::CustodyDurableRecord,
    crate::paths::identity::FileIdentity,
) {
    fixture
        .state
        .namespace
        .read_json_with_identity(&fixture.state.paths.pty_custody(fixture.terminal_id()))
        .unwrap()
}

#[cfg(target_os = "macos")]
async fn reconcile_committed_adoption(fixture: &Fixture) -> Result<()> {
    let permit = fixture.state.admit_system_mutation().await;
    fixture.state.reconcile_committed_custody(&permit)
}

#[cfg(target_os = "macos")]
async fn assert_committed_adoption_reconciles(boundary: CommittedAdoptionFailure) {
    let fixture = Fixture::new();
    let original_generation = fixture.state.controller_generation.load(Ordering::Acquire);
    let _wire = fail_committed_adoption(&fixture, boundary).await;
    let artifact = committed_custody_artifact(&fixture);
    let identity = secondary_custody_authority::ReadyWorkerCustodyIdentity::from_durable_for_test(
        artifact.0.armed_lease().unwrap(),
    );
    let runtime_path = fixture.state.paths.runtime_record(fixture.terminal_id());
    let runtime: (RuntimeRecord, crate::paths::identity::FileIdentity) = fixture
        .state
        .namespace
        .read_json_with_identity(&runtime_path)
        .unwrap();
    reconcile_committed_adoption(&fixture).await.unwrap();
    assert!(
        fixture
            .state
            .secondary_custody_authority
            .disconnected_for_test(identity)
    );
    super::secondary_custody_predecessor::validate_durable(Some(identity), &artifact.0).unwrap();
    assert!(
        !fixture
            .state
            .custody_authorizes(ControllerGeneration(original_generation))
    );
    assert!(
        !fixture
            .state
            .custody_authorizes(identity.controller_generation())
    );
    assert!(
        fixture
            .state
            .pty_alias_custody
            .release_identity_for_test()
            .is_none()
    );
    assert!(lock_recover(&fixture.state.secondary_custody).is_some());
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        runtime.0.controller_generation
    );
    reconcile_committed_adoption(&fixture).await.unwrap();
    assert!(committed_custody_artifact(&fixture) == artifact);
    let unchanged: (RuntimeRecord, crate::paths::identity::FileIdentity) = fixture
        .state
        .namespace
        .read_json_with_identity(&runtime_path)
        .unwrap();
    assert!(unchanged == runtime);
}
