use super::{Paths, WorkerLaunchSpec, WorkerNamespace};
use crate::process::ProcessIdentity;
use anyhow::Result;

#[path = "startup_checkpoint_active.rs"]
mod active;
#[path = "startup_checkpoint_authority.rs"]
mod authority;
#[path = "startup_checkpoint_constants.rs"]
mod constants;
#[path = "startup_checkpoint_custody_authority.rs"]
mod custody_authority;
#[path = "startup_checkpoint_fence.rs"]
mod fence;
#[path = "startup_checkpoint_io.rs"]
mod io;
#[path = "startup_checkpoint_lifecycle.rs"]
mod lifecycle;
#[path = "startup_checkpoint_mutation.rs"]
mod mutation;
#[path = "startup_checkpoint_path_io.rs"]
mod path_io;
#[path = "startup_checkpoint_predecessor.rs"]
mod predecessor;
#[path = "startup_checkpoint_read_io.rs"]
mod read_io;
#[path = "startup_checkpoint_reboot_validation.rs"]
mod reboot_validation;
#[path = "startup_checkpoint_retirement.rs"]
mod retirement;
#[path = "startup_checkpoint_schema.rs"]
mod schema;
#[path = "startup_checkpoint_settlement.rs"]
mod settlement;
#[path = "startup_checkpoint_validation.rs"]
mod validation;
pub(crate) use authority::validate_publication as validate_predecessor_publication;
use constants::STARTUP_CHECKPOINT_SCHEMA_VERSION;
pub use lifecycle::{
    worker_startup_checkpoint_allows_guardian_retirement,
    worker_startup_checkpoint_allows_lifecycle_mutation,
};
pub use predecessor::worker_startup_checkpoint_authorizes_retained_runtime;
pub(crate) use reboot_validation::source_before_reboot_migration;
pub use retirement::{
    WorkerStartupCheckpointPreparation, WorkerStartupCheckpointReplayDisposition,
    prepare_worker_startup_checkpoint_for_launch, worker_startup_checkpoint_replay_disposition,
};
#[cfg(not(feature = "host-test-hooks"))]
pub(crate) use schema::{
    StartupCheckpointEvidence, StartupLaunchFence, StartupPhase, StartupProcessIdentity,
    StartupPtyCustodyRollover, StartupStage, StartupStatus,
};
#[cfg(feature = "host-test-hooks")]
pub use schema::{
    StartupCheckpointEvidence, StartupLaunchFence, StartupPhase, StartupProcessIdentity,
    StartupPtyCustodyRollover, StartupStage, StartupStatus,
};
pub use settlement::{
    WorkerStartupCheckpointPolicy, WorkerStartupCheckpointSettlement,
    settle_worker_startup_checkpoint_for_launch,
};
#[cfg(any(test, feature = "host-test-hooks"))]
pub const TEST_WORKER_STARTUP_CHECKPOINT_SCHEMA_VERSION: u16 = STARTUP_CHECKPOINT_SCHEMA_VERSION;

#[cfg(any(test, feature = "host-test-hooks"))]
pub fn worker_startup_launch_fence_for_test(
    launch: &WorkerLaunchSpec,
) -> Result<StartupLaunchFence> {
    StartupLaunchFence::from_launch(launch)
}

pub(crate) struct ActiveStartupCheckpoint {
    pub(crate) evidence: StartupCheckpointEvidence,
    pub(crate) checkpoint_identity: crate::paths::identity::FileIdentity,
    pub(crate) launch_identity: crate::paths::identity::FileIdentity,
    pub(crate) predecessor_authority: Option<StartupPredecessorAuthority>,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StartupPredecessorAuthority {
    pub(crate) checkpoint_identity: crate::paths::identity::FileIdentity,
    pub(crate) runtime_sha256: crate::sha256_digest::Sha256Digest,
    pub(crate) custody: StartupPredecessorCustodyAuthority,
    pub(crate) successor: StartupLaunchFence,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum StartupPredecessorCustodyAuthority {
    Absent,
    Present {
        identity: crate::paths::identity::FileIdentity,
        sha256: crate::sha256_digest::Sha256Digest,
    },
}

impl ActiveStartupCheckpoint {
    pub(crate) fn begin(
        namespace: &WorkerNamespace,
        paths: &Paths,
        launch: &WorkerLaunchSpec,
        worker: ProcessIdentity,
    ) -> Result<Self> {
        let launch_fence = StartupLaunchFence::from_launch(launch)?;
        let evidence = StartupCheckpointEvidence {
            schema_version: STARTUP_CHECKPOINT_SCHEMA_VERSION,
            terminal_id: launch.terminal_id,
            runtime_epoch: launch.runtime_epoch,
            worker_id: launch.worker_id,
            controller_generation: launch.controller_generation,
            launch_sha256: launch_fence.launch_sha256,
            boot_id: crate::process::boot_id(),
            worker: StartupProcessIdentity::new(worker.pid, worker.start_time),
            child: None,
            stage: StartupStage::LaunchContract,
            phase: StartupPhase::Completed,
            status: StartupStatus::Active,
            successor: None,
            predecessor_runtime_sha256: None,
            pending_pty_custody_rollover: None,
        };
        validation::validate(&evidence)?;
        let (disk_launch, launch_identity): (WorkerLaunchSpec, _) =
            namespace.read_json_with_identity(&paths.worker_launch(launch.terminal_id))?;
        anyhow::ensure!(disk_launch == *launch, constants::SUCCESSOR_ERROR);
        let (evidence, checkpoint_identity, predecessor_authority) =
            io::begin_after_lifecycle_lock(namespace, paths, &evidence)?;
        Ok(Self {
            evidence,
            checkpoint_identity,
            launch_identity,
            predecessor_authority,
        })
    }

    pub(crate) fn matches_launch(
        &self,
        launch: &WorkerLaunchSpec,
        identity: crate::paths::identity::FileIdentity,
    ) -> Result<bool> {
        Ok(self.evidence.status == StartupStatus::Active
            && self.launch_identity == identity
            && fence::from_evidence(&self.evidence) == StartupLaunchFence::from_launch(launch)?)
    }

    pub(crate) fn require_exact(&self, namespace: &WorkerNamespace, paths: &Paths) -> Result<()> {
        let (evidence, identity) = io::read(
            namespace.file(&paths.worker_startup_checkpoint(self.evidence.terminal_id))?,
            || namespace.verify(),
        )?;
        validation::validate(&evidence)?;
        anyhow::ensure!(
            evidence == self.evidence && identity == self.checkpoint_identity,
            constants::STALE_ERROR
        );
        Ok(())
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/startup_checkpoint/tests.rs"]
mod tests;
