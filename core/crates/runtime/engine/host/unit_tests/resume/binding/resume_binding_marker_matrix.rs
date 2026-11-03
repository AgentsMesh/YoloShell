use crate::host::tests::*;
use super::*;

const BINDING_MARKER_SCHEMA_INCREMENT: u16 = 1;

#[test]
fn pending_binding_clear_converges_after_the_binding_was_removed() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let binding = host_binding(manifest.terminal_id);
    let transaction = binding_clear_transaction(&host, &manifest, Some(binding.clone()));
    let marker = binding_clear_marker(&transaction, ResumeBindingClearPhase::Pending).unwrap();
    host.write_namespace_json(
        &host.paths.resume_binding_clear(manifest.terminal_id),
        &marker,
    )
    .unwrap();

    host.stage_resume_binding_disposition(&transaction).unwrap();

    let persisted: ResumeBindingClearMarker =
        read_json(&host.paths.resume_binding_clear(manifest.terminal_id)).unwrap();
    assert_eq!(persisted.phase, ResumeBindingClearPhase::Cleared);
}

#[test]
fn binding_marker_cas_preserves_a_validated_replacement() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let transaction = binding_clear_transaction(&host, &manifest, None);
    let expected = binding_clear_marker(&transaction, ResumeBindingClearPhase::Pending).unwrap();
    let cleared = binding_clear_marker(&transaction, ResumeBindingClearPhase::Cleared).unwrap();
    let competitor_transaction = binding_clear_transaction(&host, &manifest, None);
    let competitor =
        binding_clear_marker(&competitor_transaction, ResumeBindingClearPhase::Pending).unwrap();
    let path = host.paths.resume_binding_clear(manifest.terminal_id);
    host.write_namespace_json(&path, &expected).unwrap();

    let replaced = host.replace_resume_binding_marker_with_boundary(&expected, &cleared, || {
        host.write_namespace_json(&path, &competitor)
    });

    assert!(replaced.is_err());
    assert_eq!(
        read_json::<ResumeBindingClearMarker>(&path).unwrap(),
        competitor
    );
}

#[test]
fn clear_without_previous_preserves_a_binding_published_after_staging() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let transaction = binding_clear_transaction(&host, &manifest, None);
    host.stage_resume_binding_disposition(&transaction).unwrap();
    let replacement = host_binding(manifest.terminal_id);
    let store = AgentBindingStore::new(host.paths.agent_binding(manifest.terminal_id));
    store.save(&replacement).unwrap();

    host.stage_resume_binding_disposition(&transaction).unwrap();
    host.apply_resume_binding_disposition(&transaction).unwrap();
    host.complete_resume_binding_disposition(&transaction)
        .unwrap();

    assert_eq!(
        read_json::<AgentBinding>(store.path()).unwrap(),
        replacement
    );
}

#[test]
fn pending_binding_clear_cannot_be_committed_or_consumed() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let transaction = binding_clear_transaction(&host, &manifest, None);
    let marker = binding_clear_marker(&transaction, ResumeBindingClearPhase::Pending).unwrap();
    assert!(
        host.complete_resume_binding_disposition(&transaction)
            .is_ok()
    );
    host.write_namespace_json(
        &host.paths.resume_binding_clear(manifest.terminal_id),
        &marker,
    )
    .unwrap();

    assert!(host.apply_resume_binding_disposition(&transaction).is_err());
    assert!(
        host.complete_resume_binding_disposition(&transaction)
            .is_err()
    );
}

#[test]
fn binding_clear_marker_identity_fields_are_all_required() {
    let terminal_id = Uuid::new_v4();
    let binding = host_binding(terminal_id);
    let previous = Some(binding.clone());
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let transaction = binding_clear_transaction(&host, &manifest, previous.clone());
    let marker = binding_clear_marker(&transaction, ResumeBindingClearPhase::Pending).unwrap();
    assert!(marker.validate_for(&transaction).is_ok());

    let mut invalid = marker.clone();
    invalid.schema_version = invalid
        .schema_version
        .saturating_add(BINDING_MARKER_SCHEMA_INCREMENT);
    assert!(invalid.validate_for(&transaction).is_err());
    invalid = marker.clone();
    invalid.terminal_id = Uuid::new_v4();
    assert!(invalid.validate_for(&transaction).is_err());
    invalid = marker.clone();
    invalid.operation_id = Uuid::new_v4();
    assert!(invalid.validate_for(&transaction).is_err());
    invalid = marker.clone();
    invalid.planned_runtime_epoch = Uuid::new_v4();
    assert!(invalid.validate_for(&transaction).is_err());
    invalid = marker.clone();
    invalid.transaction_digest.clear();
    assert!(invalid.validate_for(&transaction).is_err());
    invalid = marker;
    invalid.previous = None;
    assert!(invalid.validate_for(&transaction).is_err());
}

#[test]
fn non_clear_resume_rejects_an_unexpected_binding_marker() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let clear = binding_clear_transaction(&host, &manifest, None);
    let marker = binding_clear_marker(&clear, ResumeBindingClearPhase::Pending).unwrap();
    host.write_namespace_json(
        &host.paths.resume_binding_clear(manifest.terminal_id),
        &marker,
    )
    .unwrap();

    assert!(host.stage_resume_binding_disposition(&transaction).is_err());
    assert!(host.apply_resume_binding_disposition(&transaction).is_err());
    assert!(
        host.complete_resume_binding_disposition(&transaction)
            .is_err()
    );
}

pub(super) fn binding_clear_transaction(
    host: &Host,
    manifest: &TerminalManifest,
    previous: Option<AgentBinding>,
) -> ResumeTransaction {
    let previous = previous.map(|binding| {
        host.paths.ensure_session(manifest.terminal_id).unwrap();
        let path = host.paths.agent_binding(manifest.terminal_id);
        let present = path.exists();
        if !present {
            host.write_namespace_json(&path, &binding).unwrap();
        }
        assert_eq!(read_json::<AgentBinding>(&path).unwrap(), binding);
        let evidence = host
            .capture_resume_binding_evidence(&path)
            .unwrap()
            .unwrap();
        if !present {
            host.clear_resume_binding_evidence(&path, &evidence)
                .unwrap();
        }
        evidence
    });
    let launch = shell_resume_launch(host, manifest);
    ResumeTransaction::new(
        Uuid::new_v4(),
        ResumeIntent::Explicit,
        ResumeTarget::Shell {
            binding: ResumeBindingDisposition::Clear {
                operation_id: Uuid::new_v4(),
                previous,
                codex_authority: None,
            },
        },
        &launch,
    )
    .unwrap()
}
