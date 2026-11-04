const REMOVAL_RESTORED_MANIFEST: &str = "restored-session-manifest";
const REMOVAL_UNSAFE_DIRECTORY_MODE: u32 = 0o755;

#[tokio::test]
async fn restored_manifest_while_waiting_for_mutation_does_not_terminate() {
    let fixture = Fixture::new();
    let manifest = fixture.state.paths.manifest(fixture.terminal_id());
    let retained = fixture._filesystem.scratch_path(REMOVAL_RESTORED_MANIFEST);
    std::fs::rename(&manifest, &retained).unwrap();
    let removal = into_session_removal(observe_session_removal(&fixture.state))
        .unwrap()
        .unwrap();
    let held = fixture.state.admit_system_mutation().await;
    let mut termination = Box::pin(removal.terminate(&fixture.state, fixture.terminal_id()));
    assert!(poll_removal_once(termination.as_mut()).is_pending());

    std::fs::rename(&retained, &manifest).unwrap();
    drop(held);

    assert_eq!(
        termination.await.unwrap(),
        SessionRemovalSettlement::RetryObservation
    );
    assert_removal_did_not_terminate(&fixture);
    assert!(matches!(
        observe_session_removal(&fixture.state),
        SessionRemovalObservation::Present
    ));
    fixture
        .state
        .admit_controller_mutation(fixture.controller())
        .await
        .unwrap();
}

#[tokio::test]
async fn indeterminate_manifest_revalidation_releases_permit_without_shutdown() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    let manifest = fixture.state.paths.manifest(fixture.terminal_id());
    let retained = fixture._filesystem.scratch_path(REMOVAL_RESTORED_MANIFEST);
    std::fs::rename(&manifest, &retained).unwrap();
    let removal = into_session_removal(observe_session_removal(&fixture.state))
        .unwrap()
        .unwrap();
    let directory = fixture.state.paths.session_dir(fixture.terminal_id());
    let permissions = std::fs::metadata(&directory).unwrap().permissions();
    let held = fixture.state.admit_system_mutation().await;
    let mut termination = Box::pin(removal.terminate(&fixture.state, fixture.terminal_id()));
    assert!(poll_removal_once(termination.as_mut()).is_pending());
    std::fs::set_permissions(
        &directory,
        std::fs::Permissions::from_mode(REMOVAL_UNSAFE_DIRECTORY_MODE),
    )
    .unwrap();
    assert!(matches!(
        fixture.state.namespace.verification(),
        NamespaceVerification::Indeterminate(_)
    ));
    drop(held);
    let result = termination.await;
    std::fs::set_permissions(&directory, permissions).unwrap();
    std::fs::rename(&retained, &manifest).unwrap();

    assert_eq!(result.unwrap(), SessionRemovalSettlement::RetryObservation);
    assert_removal_did_not_terminate(&fixture);
    fixture
        .state
        .admit_controller_mutation(fixture.controller())
        .await
        .unwrap();
}

#[tokio::test]
async fn still_missing_manifest_proceeds_to_exact_termination() {
    let fixture = Fixture::new();
    fixture.state.reader_lifecycle.mark_drained();
    let manifest = fixture.state.paths.manifest(fixture.terminal_id());
    std::fs::remove_file(&manifest).unwrap();
    let removal = into_session_removal(observe_session_removal(&fixture.state))
        .unwrap()
        .unwrap();

    assert_eq!(
        removal
            .terminate(&fixture.state, fixture.terminal_id())
            .await
            .unwrap(),
        SessionRemovalSettlement::Terminated
    );
    assert_eq!(fixture.state.termination_reason(), Some(ExitReason::Lost));
}

#[test]
fn namespace_loss_proof_stays_authoritative_after_the_path_is_restored() {
    let fixture = Fixture::new();
    let home = fixture.state.paths.home().to_path_buf();
    let retained = fixture
        ._filesystem
        .scratch_path(SESSION_REMOVAL_DISPLACED_HOME);
    std::fs::rename(&home, &retained).unwrap();
    let removal = into_session_removal(observe_session_removal(&fixture.state))
        .unwrap()
        .unwrap();
    std::fs::rename(&retained, &home).unwrap();

    assert!(matches!(removal, SessionRemoval::NamespaceLost { .. }));
    assert!(removal.remains_removed(&fixture.state).unwrap());
}

#[tokio::test]
async fn foreign_manifest_path_cannot_authorize_worker_termination() {
    let fixture = Fixture::new();
    let foreign = fixture.state.paths.manifest(Uuid::new_v4());
    let result = SessionRemoval::ManifestMissing(foreign)
        .terminate(&fixture.state, fixture.terminal_id())
        .await
        .unwrap();

    assert_eq!(result, SessionRemovalSettlement::RetryObservation);
    assert_removal_did_not_terminate(&fixture);
}

fn assert_removal_did_not_terminate(fixture: &Fixture) {
    assert_eq!(fixture.state.termination_reason(), None);
    assert!(!fixture.state.reader_lifecycle.is_cancelled());
    assert!(!fixture.state.child_waited.load(Ordering::Acquire));
    assert!(!fixture.state.response_shutdown_is_committed());
}
