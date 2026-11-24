use crate::host::tests::*;
use super::*;

#[test]
fn binding_provenance_rejects_stale_cleared_marker_from_independent_transaction() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let first = binding_clear_transaction(&host, &manifest, None);
    host.stage_resume_binding_disposition(&first).unwrap();
    let marker_path = host.paths.resume_binding_clear(manifest.terminal_id);
    let stale: ResumeBindingClearMarker = read_json(&marker_path).unwrap();
    let second = independent_binding_clear_successor(&host, &manifest, &first);

    assert!(host.stage_resume_binding_disposition(&second).is_err());
    assert!(host.apply_resume_binding_disposition(&second).is_err());
    assert!(host.complete_resume_binding_disposition(&second).is_err());
    assert_eq!(
        read_json::<ResumeBindingClearMarker>(&marker_path).unwrap(),
        stale
    );
}

#[test]
fn binding_provenance_never_rolls_pending_marker_across_transactions() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let first = binding_clear_transaction(&host, &manifest, None);
    let pending = binding_clear_marker(&first, ResumeBindingClearPhase::Pending).unwrap();
    let marker_path = host.paths.resume_binding_clear(manifest.terminal_id);
    host.write_namespace_json(&marker_path, &pending).unwrap();
    let second = lineage_binding_clear_successor(&host, &manifest, &first);

    assert!(host.stage_resume_binding_disposition(&second).is_err());
    assert_eq!(
        read_json::<ResumeBindingClearMarker>(&marker_path).unwrap(),
        pending
    );
}

#[test]
fn binding_provenance_rolls_cleared_marker_to_exact_lineage_successor() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let first = binding_clear_transaction(&host, &manifest, None);
    host.stage_resume_binding_disposition(&first).unwrap();
    let second = lineage_binding_clear_successor(&host, &manifest, &first);

    host.stage_resume_binding_disposition(&second).unwrap();

    let marker_path = host.paths.resume_binding_clear(manifest.terminal_id);
    let rolled: ResumeBindingClearMarker = read_json(&marker_path).unwrap();
    assert_eq!(rolled.phase, ResumeBindingClearPhase::Cleared);
    assert_eq!(rolled.planned_runtime_epoch, second.planned_runtime_epoch);
    assert_eq!(rolled.transaction_digest, second.transaction_digest);
    rolled.validate_for(&second).unwrap();
    assert!(host.complete_resume_binding_disposition(&first).is_err());
    assert_eq!(
        read_json::<ResumeBindingClearMarker>(&marker_path).unwrap(),
        rolled
    );
}

#[test]
fn binding_provenance_crash_replay_preserves_a_new_binding() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let original = host_binding(manifest.terminal_id);
    let store = AgentBindingStore::new(host.paths.agent_binding(manifest.terminal_id));
    store.save(&original).unwrap();
    let transaction = binding_clear_transaction(&host, &manifest, Some(original.clone()));
    let marker_path = host.paths.resume_binding_clear(manifest.terminal_id);
    let pending = binding_clear_marker(&transaction, ResumeBindingClearPhase::Pending).unwrap();
    host.write_namespace_json(&marker_path, &pending).unwrap();
    assert_eq!(read_json::<AgentBinding>(store.path()).unwrap(), original);

    host.stage_resume_binding_disposition(&transaction).unwrap();
    assert!(!store.path().exists());
    let cleared: ResumeBindingClearMarker = read_json(&marker_path).unwrap();
    assert_eq!(cleared.phase, ResumeBindingClearPhase::Cleared);
    let mut replacement = original;
    replacement.updated_at_ms = replacement.updated_at_ms.saturating_add(1);
    store.save(&replacement).unwrap();

    host.stage_resume_binding_disposition(&transaction).unwrap();
    host.apply_resume_binding_disposition(&transaction).unwrap();
    assert_eq!(
        read_json::<AgentBinding>(store.path()).unwrap(),
        replacement
    );
}

#[test]
fn binding_provenance_rejects_a_nil_operation_identity() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut transaction = binding_clear_transaction(&host, &manifest, None);
    match &mut transaction.target {
        ResumeTarget::Shell {
            binding: ResumeBindingDisposition::Clear { operation_id, .. },
        } => *operation_id = Uuid::nil(),
        _ => unreachable!(),
    }
    transaction.transaction_digest = resume_transaction_digest(&transaction).unwrap();

    assert!(transaction.validate_for(manifest.terminal_id).is_err());
}

fn lineage_binding_clear_successor(
    host: &Host,
    manifest: &TerminalManifest,
    predecessor: &ResumeTransaction,
) -> ResumeTransaction {
    binding_clear_successor(host, manifest, predecessor, predecessor.target.clone())
}

fn independent_binding_clear_successor(
    host: &Host,
    manifest: &TerminalManifest,
    predecessor: &ResumeTransaction,
) -> ResumeTransaction {
    binding_clear_successor(
        host,
        manifest,
        predecessor,
        ResumeTarget::Shell {
            binding: ResumeBindingDisposition::Clear {
                operation_id: Uuid::new_v4(),
                previous: None,
                codex_authority: None,
            },
        },
    )
}

fn binding_clear_successor(
    host: &Host,
    manifest: &TerminalManifest,
    predecessor: &ResumeTransaction,
    target: ResumeTarget,
) -> ResumeTransaction {
    let launch = shell_resume_launch(host, manifest);
    ResumeTransaction::new(
        predecessor.planned_runtime_epoch,
        ResumeIntent::Explicit,
        target,
        &launch,
    )
    .unwrap()
}
