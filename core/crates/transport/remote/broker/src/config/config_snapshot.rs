use super::IdentityPassphrases;
use super::broker_paths::BrokerPaths;
use super::config_generation_pending::{GenerationRoot, PendingDirectory, entry_name};
use super::constants::*;
use super::handoff::BrokerReconcileGuard;
use anyhow::{Context, Result, ensure};
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

#[path = "config_snapshot_material.rs"]
mod material;
#[path = "config_snapshot_verify.rs"]
mod verify;
pub(super) use material::revision_material;
use material::{SnapshotMaterial, identity_name, rewrite_paths};
use verify::verified_generation;
#[path = "config_snapshot_types.rs"]
mod types;
pub use types::BrokerConfigGeneration;
pub(super) use types::GenerationPublishStage;

pub fn stage_broker_config(source: &Path, paths: &BrokerPaths) -> Result<BrokerConfigGeneration> {
    stage_broker_config_with_hook(source, paths, |_| Ok(()))
}

pub fn stage_broker_config_guarded(
    source: &Path,
    paths: &BrokerPaths,
    guard: &BrokerReconcileGuard,
) -> Result<BrokerConfigGeneration> {
    stage_broker_config_guarded_with_passphrases_hook(
        source,
        paths,
        guard,
        &IdentityPassphrases::none(),
        |_| Ok(()),
    )
}

#[cfg(test)]
fn stage_broker_config_guarded_with_hook<F>(
    source: &Path,
    paths: &BrokerPaths,
    guard: &BrokerReconcileGuard,
    hook: F,
) -> Result<BrokerConfigGeneration>
where
    F: FnMut(GenerationPublishStage) -> Result<()>,
{
    stage_broker_config_guarded_with_passphrases_hook(
        source,
        paths,
        guard,
        &IdentityPassphrases::none(),
        hook,
    )
}

pub(super) fn stage_broker_config_guarded_with_passphrases_hook<F>(
    source: &Path,
    paths: &BrokerPaths,
    guard: &BrokerReconcileGuard,
    passphrases: &IdentityPassphrases,
    hook: F,
) -> Result<BrokerConfigGeneration>
where
    F: FnMut(GenerationPublishStage) -> Result<()>,
{
    guard.verify()?;
    let root = GenerationRoot::ensure_retained(guard.state_directory()?, &paths.generations())?;
    let generation = stage_with_root(source, root, passphrases, hook);
    guard.verify()?;
    generation
}

fn stage_broker_config_with_hook<F>(
    source: &Path,
    paths: &BrokerPaths,
    hook: F,
) -> Result<BrokerConfigGeneration>
where
    F: FnMut(GenerationPublishStage) -> Result<()>,
{
    let root = GenerationRoot::ensure(&paths.generations())?;
    stage_with_root(source, root, &IdentityPassphrases::none(), hook)
}

fn stage_with_root<F>(
    source: &Path,
    root: GenerationRoot,
    passphrases: &IdentityPassphrases,
    mut hook: F,
) -> Result<BrokerConfigGeneration>
where
    F: FnMut(GenerationPublishStage) -> Result<()>,
{
    let SnapshotMaterial {
        mut raw,
        known_hosts,
        identities,
        artifact,
        revision,
    } = material::read(source, passphrases)?;
    let destination_name = entry_name(&revision)?;
    let destination = root.path().join(&revision);
    if let Some(opened) = root.open_generation(&destination_name)? {
        confirm_existing(&root, &mut hook)?;
        return verified_generation(
            &root,
            &destination_name,
            opened,
            destination,
            revision,
            passphrases,
            &mut hook,
        );
    }

    let mut pending = PendingDirectory::create(&root)?;
    rewrite_paths(&mut raw, &destination);
    pending.write_file(&entry_name(GENERATION_KNOWN_HOSTS_FILE)?, &known_hosts)?;
    for (index, bytes) in identities.iter().enumerate() {
        pending.write_file(&entry_name(identity_name(index))?, bytes)?;
    }
    if let Some(bytes) = artifact.as_deref() {
        pending.write_file(&entry_name(GENERATION_ARTIFACT_FILE)?, bytes)?;
    }
    let encoded = serde_json::to_vec(&raw).context("encode broker config snapshot")?;
    pending.write_file(&entry_name(GENERATION_CONFIG_FILE)?, &encoded)?;
    pending.seal(&root)?;
    hook(GenerationPublishStage::SourceVerified)?;

    let published = pending.publish(&root, &destination_name)?;
    let opened = if published {
        hook(GenerationPublishStage::DestinationRenamed)?;
        root.sync_and_verify()?;
        let opened = root
            .open_generation(&destination_name)?
            .context("published broker config generation disappeared")?;
        ensure!(
            pending.matches(&opened),
            "published broker generation identity changed"
        );
        hook(GenerationPublishStage::DestinationCommitted)?;
        opened
    } else {
        pending.verify_source(&root, IMMUTABLE_DIRECTORY_MODE)?;
        confirm_existing(&root, &mut hook)?;
        root.open_generation(&destination_name)?
            .context("published broker config generation disappeared")?
    };
    let generation = verified_generation(
        &root,
        &destination_name,
        opened,
        destination,
        revision,
        passphrases,
        &mut hook,
    )?;
    Ok(generation)
}

fn confirm_existing<F>(root: &GenerationRoot, hook: &mut F) -> Result<()>
where
    F: FnMut(GenerationPublishStage) -> Result<()>,
{
    root.sync_and_verify()?;
    hook(GenerationPublishStage::ExistingDurabilityConfirmed)
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/config/config_snapshot_private_tests.rs"]
mod tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/config/config_generation_publish_tests.rs"]
mod publish_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/config/config_generation_reuse_tests.rs"]
mod reuse_tests;

#[cfg(all(test, target_os = "macos", feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/config/config_generation_acl_tests.rs"]
mod acl_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/config/config_generation_namespace_tests.rs"]
mod namespace_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/config/config_snapshot_failure_tests.rs"]
mod failure_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/config/config_snapshot_instance_matrix.rs"]
mod instance_matrix;
