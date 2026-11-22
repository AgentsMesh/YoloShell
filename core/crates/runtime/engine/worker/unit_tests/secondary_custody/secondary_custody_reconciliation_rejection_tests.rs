use crate::worker::tests::*;
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn same_value_candidate_file_replacement_cannot_reconcile_committed_adoption() {
    let fixture = Fixture::new();
    let _wire =
        fail_committed_adoption(&fixture, CommittedAdoptionFailure::ReadyAcknowledgementLost).await;
    let artifact = committed_custody_artifact(&fixture);
    let path = fixture.state.paths.pty_custody(fixture.terminal_id());
    fixture
        .state
        .namespace
        .write_json(&path, &artifact.0)
        .unwrap();
    assert!(reconcile_committed_adoption(&fixture).await.is_err());
    assert!(reconcile_committed_adoption(&fixture).await.is_err());
    assert!(!fixture.state.custody_authorizes(ControllerGeneration(
        artifact.0.armed_lease().unwrap().controller_generation
    )));
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn changed_candidate_identities_cannot_reconcile_committed_adoption() {
    let mutations: &[fn(&mut crate::pty_custody::CustodyDurableLease)] = &[
        |lease| lease.lease_id = crate::pty_custody::CustodyLeaseId::new(),
        |lease| lease.offer_id = crate::pty_custody::CustodyOfferId::new(),
        |lease| {
            lease.holder.start_time = lease
                .holder
                .start_time
                .saturating_add(CUSTODY_SUCCESSOR_INCREMENT)
        },
        |lease| {
            lease.recovery_identity_sha256 =
                crate::sha256_digest::Sha256Digest::repeated(FORGED_PREDECESSOR_DIGEST_BYTE)
        },
        |lease| lease.guardian = None,
    ];
    for mutation in mutations {
        let fixture = Fixture::new();
        let _wire =
            fail_committed_adoption(&fixture, CommittedAdoptionFailure::ReadyAcknowledgementLost)
                .await;
        let before = fixture
            .state
            .secondary_custody_authority
            .predecessor_identity();
        rewrite_predecessor(&fixture, mutation);
        assert!(reconcile_committed_adoption(&fixture).await.is_err());
        assert!(
            fixture
                .state
                .secondary_custody_authority
                .predecessor_identity()
                == before
        );
    }
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn committed_reconciliation_rejects_changed_runtime_identity() {
    let fixture = Fixture::new();
    let _wire =
        fail_committed_adoption(&fixture, CommittedAdoptionFailure::ReadyAcknowledgementLost).await;
    let artifact = committed_custody_artifact(&fixture);
    let path = fixture.state.paths.runtime_record(fixture.terminal_id());
    let mut runtime: RuntimeRecord = fixture.state.namespace.read_json(&path).unwrap();
    runtime.runtime_epoch = Uuid::new_v4();
    fixture.state.namespace.write_json(&path, &runtime).unwrap();
    assert!(reconcile_committed_adoption(&fixture).await.is_err());
    assert!(committed_custody_artifact(&fixture) == artifact);
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn committed_reconciliation_cannot_restore_explicitly_fenced_authority() {
    let fixture = Fixture::new();
    let _wire =
        fail_committed_adoption(&fixture, CommittedAdoptionFailure::ReadyAcknowledgementLost).await;
    fixture.state.fence_all_custody().await;
    assert!(reconcile_committed_adoption(&fixture).await.is_err());
    assert!(
        fixture
            .state
            .secondary_custody_authority
            .predecessor_identity()
            .is_none()
    );
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn committed_reconciliation_does_not_cross_archived_manifest_boundary() {
    let fixture = Fixture::new();
    let _wire =
        fail_committed_adoption(&fixture, CommittedAdoptionFailure::ReadyAcknowledgementLost).await;
    let path = fixture.state.paths.manifest(fixture.terminal_id());
    let mut manifest: crate::model::TerminalManifest =
        fixture.state.namespace.read_json(&path).unwrap();
    manifest.archived = true;
    fixture
        .state
        .namespace
        .write_json(&path, &manifest)
        .unwrap();
    assert!(reconcile_committed_adoption(&fixture).await.is_err());
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn committed_reconciliation_cannot_regress_an_already_observed_generation() {
    let fixture = Fixture::new();
    let _wire =
        fail_committed_adoption(&fixture, CommittedAdoptionFailure::ReadyAcknowledgementLost).await;
    let artifact = committed_custody_artifact(&fixture);
    let generation = artifact.0.armed_lease().unwrap().controller_generation;
    fixture
        .state
        .controller_generation
        .store(generation, Ordering::Release);
    fixture
        .state
        .controller_persisted_generation
        .store(generation, Ordering::Release);
    assert!(reconcile_committed_adoption(&fixture).await.is_err());
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        generation
    );
    assert_eq!(
        fixture
            .state
            .controller_persisted_generation
            .load(Ordering::Acquire),
        generation
    );
    assert!(committed_custody_artifact(&fixture) == artifact);
}
