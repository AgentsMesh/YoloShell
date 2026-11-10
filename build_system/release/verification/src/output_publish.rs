use crate::output::{OutputDirectory, OutputTransaction};
use crate::output_stage::StagedCandidate;
use crate::verify::verify_opened;
use anyhow::{Context, Result};
use yolo_release_candidate_artifact::PinnedArtifact;
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_gate_system::{ensure_missing, validate_parent};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublicationCheckpoint {
    Renamed,
    InitialSyncComplete,
    EntriesReopened,
    ContentVerified,
    ModeProtected,
    MarkerApplied,
    CandidateBarrierComplete,
    DurabilityBarriersComplete,
    CommitMarked,
    FinalValidationComplete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PublicationSync {
    Initial,
    Commit,
}

pub(crate) trait PublishHooks {
    fn sync_candidate(&self, _phase: PublicationSync, candidate: &DirectoryHandle) -> Result<()> {
        candidate.durability_barrier()
    }

    fn sync_parent(&self, _phase: PublicationSync, parent: &DirectoryHandle) -> Result<()> {
        parent.durability_barrier()
    }

    fn checkpoint(&self, _checkpoint: PublicationCheckpoint) -> Result<()> {
        Ok(())
    }
}

struct ProductionHooks;

impl PublishHooks for ProductionHooks {}

impl OutputTransaction {
    pub fn publish(
        &self,
        artifact: &PinnedArtifact,
        checksum: &[u8],
        provenance: &[u8],
    ) -> Result<OutputDirectory> {
        self.publish_with_hooks(artifact, checksum, provenance, &ProductionHooks)
    }

    pub(crate) fn publish_with_hooks(
        &self,
        artifact: &PinnedArtifact,
        checksum: &[u8],
        provenance: &[u8],
        hooks: &dyn PublishHooks,
    ) -> Result<OutputDirectory> {
        let _publication_lock = self.parent.lock_exclusive()?;
        validate_parent(&self.parent)?;
        ensure_missing(&self.parent, &self.destination)?;
        let mut stage = StagedCandidate::create(&self.parent)?;
        stage.write(artifact, checksum, provenance)?;
        let staged = OutputDirectory::from_handle(stage.handle().clone())?;
        verify_opened(&staged).context("verify staged release candidate")?;
        validate_parent(&self.parent)?;
        self.commit_stage(stage, hooks)
    }
}
