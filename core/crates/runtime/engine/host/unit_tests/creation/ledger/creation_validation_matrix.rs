use crate::host::tests::*;
use super::*;

const CREATION_MANIFEST_MISMATCH: &str = "creation-manifest-mismatch";
const CREATION_MANIFEST_ARGUMENT: &str = "manifest-mismatch-argument";
const CREATION_MANIFEST_SCHEMA_OFFSET: u16 = 1;
const CREATION_MANIFEST_TIME_OFFSET: i64 = 1;

#[tokio::test]
async fn bound_manifest_accepts_empty_and_matching_durable_pairs() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let (_, binding, _) = reserve_test_creation(&host, directory.path(), "manifest-pairs");
    assert!(
        host.existing_bound_manifest(&binding)
            .await
            .unwrap()
            .is_none()
    );

    let manifest = host.prepare_bound_session(&binding).await.unwrap();
    let existing = host
        .existing_bound_manifest(&binding)
        .await
        .unwrap()
        .unwrap();
    assert!(same_manifest(&existing, &manifest));
}

#[tokio::test]
async fn creation_manifest_matching_rejects_every_bound_field_mutation() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let (_, binding, _) = reserve_test_creation(&host, directory.path(), "manifest-fields");
    let manifest = host.prepare_bound_session(&binding).await.unwrap();
    assert!(manifest_matches_binding(&manifest, &binding));

    let mutations: [fn(&mut TerminalManifest); 9] = [
        |value| value.schema_version += CREATION_MANIFEST_SCHEMA_OFFSET,
        |value| value.terminal_id = Uuid::nil(),
        |value| value.name.push_str(CREATION_MANIFEST_MISMATCH),
        |value| value.command.push_str(CREATION_MANIFEST_MISMATCH),
        |value| value.args.push(CREATION_MANIFEST_ARGUMENT.into()),
        |value| value.cwd.push(CREATION_MANIFEST_MISMATCH),
        |value| value.cols = value.cols.saturating_add(1),
        |value| value.rows = value.rows.saturating_add(1),
        |value| value.archived = true,
    ];
    for mutate in mutations {
        let mut candidate = manifest.clone();
        mutate(&mut candidate);
        assert!(!manifest_matches_binding(&candidate, &binding));
    }

    let mut unnamed_binding = binding;
    unnamed_binding.spec.name = None;
    let mut unnamed_manifest = manifest;
    let shell = shell_command(&unnamed_binding.spec.cwd);
    unnamed_manifest.name = command_basename(shell.executable.to_string_lossy().as_ref());
    assert!(manifest_matches_binding(
        &unnamed_manifest,
        &unnamed_binding
    ));
}

#[test]
fn creation_manifest_equivalence_rejects_every_durable_field_mutation() {
    let directory = tempdir().unwrap();
    let manifest = fixture_manifest(directory.path());
    assert!(same_manifest(&manifest, &manifest));

    let mutations: [fn(&mut TerminalManifest); 12] = [
        |value| value.schema_version += CREATION_MANIFEST_SCHEMA_OFFSET,
        |value| value.terminal_id = Uuid::nil(),
        |value| value.name.push_str(CREATION_MANIFEST_MISMATCH),
        |value| value.command.push_str(CREATION_MANIFEST_MISMATCH),
        |value| value.args.push(CREATION_MANIFEST_ARGUMENT.into()),
        |value| value.cwd.push(CREATION_MANIFEST_MISMATCH),
        |value| value.cols = value.cols.saturating_add(1),
        |value| value.rows = value.rows.saturating_add(1),
        |value| value.created_at_ms += CREATION_MANIFEST_TIME_OFFSET,
        |value| value.updated_at_ms += CREATION_MANIFEST_TIME_OFFSET,
        |value| value.archived = true,
        |value| value.recovery_state = RecoveryState::Uncertain,
    ];
    for mutate in mutations {
        let mut candidate = manifest.clone();
        mutate(&mut candidate);
        assert!(!same_manifest(&manifest, &candidate));
    }
}
