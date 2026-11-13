use super::{Paths, WorkerNamespace, startup_checkpoint};
use crate::model::{RuntimeRecord, TerminalManifest, WorkerLaunchSpec};
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result};

pub(crate) const LAUNCH_CONTEXT: &str = "read exact runtime publication worker launch";
pub(crate) const LAUNCH_REJECTED: &str =
    "initial runtime publication requires its exact worker launch";
pub(crate) const MANIFEST_CONTEXT: &str = "read exact runtime publication session manifest";
pub(crate) const MANIFEST_REJECTED: &str =
    "runtime publication requires its exact current session manifest";

pub(crate) struct ManifestFence {
    pub(crate) value: TerminalManifest,
    pub(crate) identity: FileIdentity,
}

impl ManifestFence {
    pub(crate) fn capture(
        namespace: &WorkerNamespace,
        paths: &Paths,
        record: &RuntimeRecord,
    ) -> Result<Self> {
        let (value, identity) = read_manifest(namespace, paths, record.terminal_id)?;
        Ok(Self { value, identity })
    }

    pub(crate) fn require_exact(
        &self,
        namespace: &WorkerNamespace,
        paths: &Paths,
        record: &RuntimeRecord,
    ) -> Result<()> {
        let (manifest, identity) = read_manifest(namespace, paths, record.terminal_id)?;
        anyhow::ensure!(
            identity == self.identity && same_manifest(&manifest, &self.value)?,
            MANIFEST_REJECTED
        );
        Ok(())
    }
}

pub(crate) struct InitialPublicationFence {
    pub(crate) manifest: ManifestFence,
    pub(crate) launch: WorkerLaunchSpec,
    pub(crate) launch_identity: FileIdentity,
}

impl InitialPublicationFence {
    pub(crate) fn capture(
        namespace: &WorkerNamespace,
        paths: &Paths,
        record: &RuntimeRecord,
        checkpoint: &startup_checkpoint::ActiveStartupCheckpoint,
    ) -> Result<Self> {
        let manifest = ManifestFence::capture(namespace, paths, record)?;
        let (launch, launch_identity) = namespace
            .read_json_with_identity(&paths.worker_launch(record.terminal_id))
            .context(LAUNCH_CONTEXT)?;
        validate_launch(&launch, launch_identity, record, checkpoint)?;
        Ok(Self {
            manifest,
            launch,
            launch_identity,
        })
    }

    pub(crate) fn require_exact(
        &self,
        namespace: &WorkerNamespace,
        paths: &Paths,
        record: &RuntimeRecord,
        checkpoint: &startup_checkpoint::ActiveStartupCheckpoint,
    ) -> Result<()> {
        checkpoint.require_exact(namespace, paths)?;
        self.manifest.require_exact(namespace, paths, record)?;
        let (launch, identity) = namespace
            .read_json_with_identity(&paths.worker_launch(record.terminal_id))
            .context(LAUNCH_CONTEXT)?;
        validate_launch(&launch, identity, record, checkpoint)?;
        anyhow::ensure!(
            identity == self.launch_identity && launch == self.launch,
            LAUNCH_REJECTED
        );
        Ok(())
    }
}

pub(crate) fn read_manifest(
    namespace: &WorkerNamespace,
    paths: &Paths,
    terminal_id: uuid::Uuid,
) -> Result<(TerminalManifest, FileIdentity)> {
    let (manifest, identity) = namespace
        .read_json_with_identity(&paths.manifest(terminal_id))
        .context(MANIFEST_CONTEXT)?;
    validate_manifest(&manifest, terminal_id)?;
    Ok((manifest, identity))
}

pub(crate) fn validate_manifest(
    manifest: &TerminalManifest,
    terminal_id: uuid::Uuid,
) -> Result<()> {
    anyhow::ensure!(
        manifest.schema_version == crate::constants::SESSION_MANIFEST_SCHEMA_VERSION
            && manifest.terminal_id == terminal_id
            && !manifest.archived,
        MANIFEST_REJECTED
    );
    Ok(())
}

pub(crate) fn same_manifest(left: &TerminalManifest, right: &TerminalManifest) -> Result<bool> {
    Ok(left == right)
}

pub(crate) fn validate_launch(
    launch: &WorkerLaunchSpec,
    identity: FileIdentity,
    record: &RuntimeRecord,
    checkpoint: &startup_checkpoint::ActiveStartupCheckpoint,
) -> Result<()> {
    anyhow::ensure!(
        checkpoint.matches_launch(launch, identity)?
            && record.schema_version == crate::constants::RUNTIME_RECORD_SCHEMA_VERSION
            && launch.schema_version == crate::constants::WORKER_LAUNCH_SCHEMA_VERSION
            && launch.terminal_id == record.terminal_id
            && launch.runtime_epoch == record.runtime_epoch
            && launch.worker_id == record.worker_id
            && launch.controller_generation == record.controller_generation
            && launch.socket_path == record.socket_path
            && launch.auth_token == record.auth_token
            && launch.command == record.command
            && launch.args == record.args
            && launch.cwd == record.cwd
            && crate::model::normalize_terminal_columns(launch.cols) == record.cols
            && crate::model::normalize_terminal_rows(launch.rows) == record.rows,
        LAUNCH_REJECTED
    );
    Ok(())
}
