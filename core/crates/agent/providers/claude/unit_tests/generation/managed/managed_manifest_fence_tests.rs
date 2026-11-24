use super::managed_mutation_support::ManagedFixture;
use crate::paths::durable_mutation::DurableMutationFencePhase;

const MANIFEST_TIMESTAMP_INCREMENT_MS: i64 = 1;
const NOT_ARCHIVED: bool = false;

#[test]
fn active_manifest_update_at_commit_boundary_allows_creation() {
    let fixture = ManagedFixture::new(NOT_ARCHIVED, None);
    let manifest = fixture.manifest();
    install_commit(move || update_manifest(&manifest));

    fixture.begin().unwrap();
    assert!(fixture.store.path().exists());
}

#[test]
fn manifest_archive_at_commit_boundary_blocks_creation() {
    let fixture = ManagedFixture::new(NOT_ARCHIVED, None);
    let manifest = fixture.manifest();
    install_commit(move || archive_manifest(&manifest));

    assert!(fixture.begin().is_err());
    assert!(!fixture.store.path().exists());
}

fn install_commit(action: impl FnOnce() -> anyhow::Result<()> + 'static) {
    let mut action = Some(action);
    crate::agent::claude::generation::mutation::set_test_hook(move |phase| {
        if phase == DurableMutationFencePhase::Commit {
            if let Some(action) = action.take() {
                return action();
            }
        }
        Ok(())
    });
}

fn update_manifest(path: &std::path::Path) -> anyhow::Result<()> {
    let mut manifest: crate::model::TerminalManifest = crate::paths::read_json(path)?;
    manifest.updated_at_ms = manifest
        .updated_at_ms
        .saturating_add(MANIFEST_TIMESTAMP_INCREMENT_MS);
    crate::paths::atomic_write_json(path, &manifest)
}

fn archive_manifest(path: &std::path::Path) -> anyhow::Result<()> {
    let mut manifest: crate::model::TerminalManifest = crate::paths::read_json(path)?;
    manifest.archived = true;
    crate::paths::atomic_write_json(path, &manifest)
}
