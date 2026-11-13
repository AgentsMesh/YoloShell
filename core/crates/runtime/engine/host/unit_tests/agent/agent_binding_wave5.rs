use crate::host::tests::*;
pub(super) const AGENT_BINDING_WAVE5_CORRUPT: &[u8] = b"{invalid-agent-binding";
const TERMINAL_RECREATE_CLASSIFICATION_DETAIL: &str =
    "terminal recreation failure lost its typed classification";

#[test]
fn resume_selection_and_failure_conversion_preserve_explicit_policy() {
    assert_eq!(
        ResumeSelection::explicit(false),
        ResumeSelection::Preserve(ResumeIntent::Explicit)
    );
    assert_eq!(
        ResumeSelection::explicit(true),
        ResumeSelection::ExplicitRecreate
    );
    assert_eq!(
        ResumeSelection::ExplicitRecreate.intent(),
        ResumeIntent::Explicit
    );
    assert_eq!(
        ResumeSelection::Preserve(ResumeIntent::Automatic).intent(),
        ResumeIntent::Automatic
    );

    let error =
        SessionResumeFailure::AutomaticBlocked(AutomaticResumeBlock::RecoveryEvidenceChanged)
            .into_explicit_error();
    assert_eq!(error.code, crate::protocol::error_code::INTERNAL);
    assert_eq!(error.message, EXPLICIT_AUTOMATIC_BLOCK_DETAIL);
}

#[tokio::test]
async fn explicit_recreate_captures_raw_previous_binding_evidence() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let binding = host_binding(manifest.terminal_id);
    AgentBindingStore::new(host.paths.agent_binding(manifest.terminal_id))
        .save(&binding)
        .unwrap();

    let selected = host
        .binding_for_resume(&mut manifest, ResumeSelection::ExplicitRecreate)
        .await
        .unwrap();

    assert!(matches!(
        selected,
        ResumeBinding::Shell(ResumeBindingDisposition::Clear {
            previous: Some(ResumeBindingEvidence { .. }),
            ..
        })
    ));
}

#[tokio::test]
async fn corrupt_binding_is_isolated_from_resume_and_view_paths() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    atomic_write_json(&host.paths.manifest(manifest.terminal_id), &manifest).unwrap();
    lock_recover(&host.catalog).insert(&manifest).unwrap();
    write_private_bytes(
        &host.paths.agent_binding(manifest.terminal_id),
        AGENT_BINDING_WAVE5_CORRUPT,
    );

    let failure = host
        .binding_for_resume(
            &mut manifest,
            ResumeSelection::Preserve(ResumeIntent::Explicit),
        )
        .await
        .unwrap_err()
        .into_explicit_error();
    assert_eq!(
        failure.code,
        crate::agent::constants::error::BINDING_INVALID
    );
    assert_eq!(manifest.recovery_state, RecoveryState::Uncertain);

    assert!(host.binding_for_view(&mut manifest).await.0.is_none());
    assert_eq!(manifest.recovery_state, RecoveryState::Uncertain);
}

#[tokio::test]
async fn corrupt_binding_view_never_publishes_a_durable_recovery_write() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Reattached;
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    atomic_write_json(&host.paths.manifest(manifest.terminal_id), &manifest).unwrap();
    lock_recover(&host.catalog).insert(&manifest).unwrap();
    write_private_bytes(
        &host.paths.agent_binding(manifest.terminal_id),
        AGENT_BINDING_WAVE5_CORRUPT,
    );

    assert!(host.binding_for_view(&mut manifest).await.0.is_none());
    assert_eq!(manifest.recovery_state, RecoveryState::Uncertain);
    let durable: TerminalManifest = read_json(&host.paths.manifest(manifest.terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Reattached);
}

#[tokio::test]
async fn plain_shell_preserve_requires_explicit_recreation() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    for intent in [ResumeIntent::Automatic, ResumeIntent::Explicit] {
        let mut manifest = fixture_manifest(directory.path());
        seed_recovery_manifest(&host, &manifest).await;
        let failure = host
            .binding_for_resume(&mut manifest, ResumeSelection::Preserve(intent))
            .await
            .unwrap_err();
        let SessionResumeFailure::TerminalRecreate(error) = failure else {
            panic!("{TERMINAL_RECREATE_CLASSIFICATION_DETAIL}")
        };
        assert_eq!(error.code, crate::protocol::error_code::RECREATE_REQUIRED);
        assert_eq!(manifest.recovery_state, RecoveryState::NeedsRecreate);
        assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
    }
}

#[tokio::test]
async fn unavailable_provider_requires_typed_terminal_recreation() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    atomic_write_json(&host.paths.manifest(manifest.terminal_id), &manifest).unwrap();
    lock_recover(&host.catalog).insert(&manifest).unwrap();
    let mut binding = completion_provider_binding(&directory, manifest.terminal_id);
    binding.state = crate::agent::AgentState::Unknown;
    binding.recovery_availability = crate::agent::AgentRecoveryAvailability::Unavailable;
    AgentBindingStore::new(host.paths.agent_binding(manifest.terminal_id))
        .save(&binding)
        .unwrap();

    let failure = host
        .binding_for_resume(
            &mut manifest,
            ResumeSelection::Preserve(ResumeIntent::Automatic),
        )
        .await
        .unwrap_err();
    let SessionResumeFailure::TerminalRecreate(error) = failure else {
        panic!("{TERMINAL_RECREATE_CLASSIFICATION_DETAIL}")
    };
    assert_eq!(
        error.code,
        crate::protocol::error_code::AGENT_RECOVERY_UNAVAILABLE
    );
    assert_eq!(manifest.recovery_state, RecoveryState::NeedsRecreate);
}

#[tokio::test]
async fn failed_terminal_state_publication_degrades_to_operational_uncertainty() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    atomic_write_json(&host.paths.manifest(manifest.terminal_id), &manifest).unwrap();
    lock_recover(&host.catalog).insert(&manifest).unwrap();
    *lock_recover(&host.catalog_test_probes.upsert_failure) = true;

    let failure = host
        .binding_for_resume(
            &mut manifest,
            ResumeSelection::Preserve(ResumeIntent::Automatic),
        )
        .await
        .unwrap_err();

    assert!(matches!(failure, SessionResumeFailure::Protocol(_)));
    assert_eq!(
        read_json::<TerminalManifest>(&host.paths.manifest(manifest.terminal_id))
            .unwrap()
            .recovery_state,
        RecoveryState::Uncertain
    );
}
