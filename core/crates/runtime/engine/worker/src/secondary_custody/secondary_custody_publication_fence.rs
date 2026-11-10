use super::{Paths, WorkerNamespace, runtime_publication_fence, startup_checkpoint};
use crate::model::{RuntimeRecord, WorkerLaunchSpec};
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::Result;

#[path = "secondary_custody_publication_fence_custody.rs"]
mod custody;
use custody::CustodyState;

const FENCE_ERROR: &str = "pending PTY custody publication evidence changed";

pub(crate) struct PublicationFence {
    runtime_identity: FileIdentity,
    checkpoint: startup_checkpoint::StartupCheckpointEvidence,
    checkpoint_identity: FileIdentity,
    launch: WorkerLaunchSpec,
    launch_identity: FileIdentity,
    manifest: runtime_publication_fence::ManifestFence,
    custody: CustodyState,
}

impl PublicationFence {
    pub(crate) fn capture(
        namespace: &WorkerNamespace,
        paths: &Paths,
        runtime: &RuntimeRecord,
        pending: &CustodyDurableRecord,
        checkpoint: &startup_checkpoint::ActiveStartupCheckpoint,
    ) -> Result<Self> {
        let (stored_runtime, runtime_identity): (RuntimeRecord, _) =
            namespace.read_json_with_identity(&paths.runtime_record(runtime.terminal_id))?;
        anyhow::ensure!(stored_runtime == *runtime, FENCE_ERROR);
        let checkpoint_path = paths.worker_startup_checkpoint(runtime.terminal_id);
        let (stored_checkpoint, checkpoint_identity): (
            startup_checkpoint::StartupCheckpointEvidence,
            _,
        ) = namespace.read_json_with_identity(&checkpoint_path)?;
        checkpoint.require_exact(namespace, paths)?;
        validate_checkpoint(&stored_checkpoint, runtime)?;
        let launch_path = paths.worker_launch(runtime.terminal_id);
        let (launch, launch_identity) = namespace.read_json_with_identity(&launch_path)?;
        anyhow::ensure!(
            checkpoint.matches_launch(&launch, launch_identity)?,
            FENCE_ERROR
        );
        let manifest =
            runtime_publication_fence::ManifestFence::capture(namespace, paths, runtime)?;
        let custody = custody::capture(namespace, paths, &stored_checkpoint, pending)?;
        Ok(Self {
            runtime_identity,
            checkpoint: stored_checkpoint,
            checkpoint_identity,
            launch,
            launch_identity,
            manifest,
            custody,
        })
    }

    pub(crate) fn already_installed(
        &self,
        namespace: &WorkerNamespace,
        paths: &Paths,
        runtime: &RuntimeRecord,
        pending: &CustodyDurableRecord,
        checkpoint: &startup_checkpoint::ActiveStartupCheckpoint,
    ) -> Result<bool> {
        if !matches!(&self.custody, CustodyState::Installed(..)) {
            return Ok(false);
        }
        self.require_exact(
            namespace,
            paths,
            runtime,
            pending,
            checkpoint,
            yolo_paths::durable_mutation::DurableMutationFencePhase::Installed,
        )?;
        Ok(true)
    }

    pub(crate) fn expected_custody_identity(&self) -> Option<FileIdentity> {
        match &self.custody {
            CustodyState::Missing => None,
            CustodyState::Source { identity, .. } => Some(*identity),
            CustodyState::Installed(..) => None,
        }
    }

    pub(crate) fn require_exact(
        &self,
        namespace: &WorkerNamespace,
        paths: &Paths,
        runtime: &RuntimeRecord,
        pending: &CustodyDurableRecord,
        checkpoint: &startup_checkpoint::ActiveStartupCheckpoint,
        phase: yolo_paths::durable_mutation::DurableMutationFencePhase,
    ) -> Result<()> {
        let (stored_runtime, identity): (RuntimeRecord, _) =
            namespace.read_json_with_identity(&paths.runtime_record(runtime.terminal_id))?;
        anyhow::ensure!(
            stored_runtime == *runtime && identity == self.runtime_identity,
            FENCE_ERROR
        );
        let (stored_checkpoint, identity): (startup_checkpoint::StartupCheckpointEvidence, _) =
            namespace
                .read_json_with_identity(&paths.worker_startup_checkpoint(runtime.terminal_id))?;
        anyhow::ensure!(
            stored_checkpoint == self.checkpoint && identity == self.checkpoint_identity,
            FENCE_ERROR
        );
        checkpoint.require_exact(namespace, paths)?;
        let (launch, identity): (WorkerLaunchSpec, _) =
            namespace.read_json_with_identity(&paths.worker_launch(runtime.terminal_id))?;
        anyhow::ensure!(
            launch == self.launch && identity == self.launch_identity,
            FENCE_ERROR
        );
        self.manifest.require_exact(namespace, paths, runtime)?;
        custody::require_exact(namespace, paths, runtime, pending, &self.custody, phase)
    }
}

fn validate_checkpoint(
    checkpoint: &startup_checkpoint::StartupCheckpointEvidence,
    runtime: &RuntimeRecord,
) -> Result<()> {
    anyhow::ensure!(
        checkpoint.status == startup_checkpoint::StartupStatus::Active
            && checkpoint.stage == startup_checkpoint::StartupStage::SecondaryCustody
            && checkpoint.phase == startup_checkpoint::StartupPhase::Entered
            && checkpoint.terminal_id == runtime.terminal_id
            && checkpoint.runtime_epoch == runtime.runtime_epoch
            && checkpoint.worker_id == runtime.worker_id
            && checkpoint.controller_generation == runtime.controller_generation,
        FENCE_ERROR
    );
    Ok(())
}
