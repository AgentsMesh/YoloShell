use crate::output::{OutputDirectory, OutputTransaction};
use crate::output_files::OpenedCandidate;
use crate::output_publish::{PublicationCheckpoint, PublicationSync, PublishHooks};
use crate::output_stage::StagedCandidate;
use crate::verify::verify_preopened;
use anyhow::{Context, Result, anyhow, ensure};
use yolo_release_contracts::{
    COMMITTED_ACKNOWLEDGEMENT_ERROR, COMMITTED_VALIDATION_ERROR, PUBLICATION_REVERIFY_ERROR,
};
use yolo_release_evidence_model::ReleaseProvenance;
use yolo_release_filesystem::{DirectoryHandle, DirectorySeal};
use yolo_release_gate_system::validate_parent;

struct CandidateBinding {
    opened: OpenedCandidate,
    expected: ReleaseProvenance,
}

struct DurableCommit {
    binding: CandidateBinding,
    barrier_output: OutputDirectory,
    parent_seal: DirectorySeal,
}

impl OutputTransaction {
    pub(crate) fn commit_stage(
        &self,
        mut stage: StagedCandidate,
        hooks: &dyn PublishHooks,
    ) -> Result<OutputDirectory> {
        stage.install_name(&self.destination)?;
        let committed = match self.commit_installed(&mut stage, hooks) {
            Ok(committed) => committed,
            Err(error) => {
                return match stage.rollback(&self.destination) {
                    Ok(()) => Err(error),
                    Err(rollback) => Err(anyhow!(
                        "{error:#}; candidate rollback failed: {rollback:#}"
                    )),
                };
            }
        };
        self.finish_committed(&committed, hooks)
    }

    fn commit_installed(
        &self,
        stage: &mut StagedCandidate,
        hooks: &dyn PublishHooks,
    ) -> Result<DurableCommit> {
        hooks.checkpoint(PublicationCheckpoint::Renamed)?;
        hooks.sync_candidate(PublicationSync::Initial, stage.handle())?;
        hooks.sync_parent(PublicationSync::Initial, &self.parent)?;
        hooks.checkpoint(PublicationCheckpoint::InitialSyncComplete)?;

        self.parent
            .verify_child(&self.destination, stage.handle())?;
        let installed = DirectoryHandle::open_at(&self.parent, &self.destination)?;
        let uncommitted = OutputDirectory::from_uncommitted(
            self.parent.clone(),
            self.destination.clone(),
            installed,
        )?;
        let opened = uncommitted.open_candidate()?;
        hooks.checkpoint(PublicationCheckpoint::EntriesReopened)?;
        let expected = verify_preopened(&uncommitted, &opened)?;
        uncommitted.verify_namespace()?;
        validate_parent(&self.parent)?;
        hooks.checkpoint(PublicationCheckpoint::ContentVerified)?;

        stage.protect_commit_mode()?;
        hooks.checkpoint(PublicationCheckpoint::ModeProtected)?;
        stage.apply_commit_marker()?;
        hooks.checkpoint(PublicationCheckpoint::MarkerApplied)?;
        let binding = CandidateBinding { opened, expected };
        let marked = DirectoryHandle::open_at(&self.parent, &self.destination)?;
        let barrier_output =
            OutputDirectory::from_bound(self.parent.clone(), self.destination.clone(), marked)?;
        self.verify_bound(&barrier_output, &binding)?;
        let parent_seal = self.parent.seal()?;
        hooks.sync_candidate(PublicationSync::Commit, &barrier_output.directory)?;
        hooks.checkpoint(PublicationCheckpoint::CandidateBarrierComplete)?;
        hooks.sync_parent(PublicationSync::Commit, &self.parent)?;
        stage.confirm_durable_commit();
        Ok(DurableCommit {
            binding,
            barrier_output,
            parent_seal,
        })
    }

    fn revalidate_committed(&self, state: &CandidateBinding) -> Result<OutputDirectory> {
        let committed = DirectoryHandle::open_at(&self.parent, &self.destination)?;
        let output =
            OutputDirectory::from_bound(self.parent.clone(), self.destination.clone(), committed)?;
        self.verify_bound(&output, state)?;
        Ok(output)
    }

    fn verify_bound(&self, output: &OutputDirectory, state: &CandidateBinding) -> Result<()> {
        let actual = verify_preopened(output, &state.opened)?;
        ensure!(actual == state.expected, PUBLICATION_REVERIFY_ERROR);
        output.verify_namespace()?;
        validate_parent(&self.parent)?;
        Ok(())
    }

    fn finish_committed(
        &self,
        state: &DurableCommit,
        hooks: &dyn PublishHooks,
    ) -> Result<OutputDirectory> {
        let barrier_acknowledgement =
            hooks.checkpoint(PublicationCheckpoint::DurabilityBarriersComplete);
        self.verify_durable_binding(state)
            .context(COMMITTED_VALIDATION_ERROR)?;
        if let Err(error) = barrier_acknowledgement {
            return Err(error).context(COMMITTED_ACKNOWLEDGEMENT_ERROR);
        }
        let marker_acknowledgement = hooks.checkpoint(PublicationCheckpoint::CommitMarked);
        self.revalidate_committed(&state.binding)
            .context(COMMITTED_VALIDATION_ERROR)?;
        if let Err(error) = marker_acknowledgement {
            return Err(error).context(COMMITTED_ACKNOWLEDGEMENT_ERROR);
        }
        let final_acknowledgement =
            hooks.checkpoint(PublicationCheckpoint::FinalValidationComplete);
        let output = self
            .revalidate_committed(&state.binding)
            .context(COMMITTED_VALIDATION_ERROR)?;
        match final_acknowledgement {
            Ok(()) => Ok(output),
            Err(error) => Err(error).context(COMMITTED_ACKNOWLEDGEMENT_ERROR),
        }
    }

    fn verify_durable_binding(&self, state: &DurableCommit) -> Result<()> {
        state.barrier_output.verify_namespace()?;
        self.parent.verify_seal(&state.parent_seal)?;
        self.verify_bound(&state.barrier_output, &state.binding)
    }
}
