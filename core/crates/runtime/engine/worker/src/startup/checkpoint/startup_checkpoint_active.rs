use super::super::{Paths, WorkerNamespace};
use super::{
    StartupCheckpointEvidence, StartupPhase, StartupPredecessorAuthority, StartupStage,
    StartupStatus, io, validation,
};
use crate::process::ProcessIdentity;
use anyhow::Result;

impl super::ActiveStartupCheckpoint {
    pub(crate) fn enter(
        &mut self,
        namespace: &WorkerNamespace,
        paths: &Paths,
        stage: StartupStage,
    ) -> Result<()> {
        validation::require_next_stage(&self.evidence, stage)?;
        let mut next = self.evidence.clone();
        next.stage = stage;
        next.phase = StartupPhase::Entered;
        self.install(namespace, paths, next)
    }

    pub(crate) fn complete(
        &mut self,
        namespace: &WorkerNamespace,
        paths: &Paths,
        stage: StartupStage,
    ) -> Result<()> {
        self.complete_with_child(namespace, paths, stage, None)
    }

    pub(crate) fn complete_child(
        &mut self,
        namespace: &WorkerNamespace,
        paths: &Paths,
        child: ProcessIdentity,
    ) -> Result<()> {
        self.complete_with_child(namespace, paths, StartupStage::ChildProcess, Some(child))
    }

    pub(crate) fn complete_with_child(
        &mut self,
        namespace: &WorkerNamespace,
        paths: &Paths,
        stage: StartupStage,
        child: Option<ProcessIdentity>,
    ) -> Result<()> {
        validation::require_entered_stage(&self.evidence, stage, child.is_some())?;
        let mut next = self.evidence.clone();
        next.phase = StartupPhase::Completed;
        if let Some(child) = child {
            next.child = Some(
                yolo_startup_checkpoint_contract::StartupProcessIdentity::new(
                    child.pid,
                    child.start_time,
                ),
            );
        }
        if stage == StartupStage::SecondaryCustody {
            next.pending_pty_custody_rollover = None;
        }
        self.install(namespace, paths, next)
    }

    pub(crate) fn retire_runtime_activated(
        &mut self,
        namespace: &WorkerNamespace,
        paths: &Paths,
    ) -> Result<()> {
        validation::require_entered_stage(&self.evidence, StartupStage::RuntimeActivation, false)?;
        let mut next = self.evidence.clone();
        next.phase = StartupPhase::Completed;
        next.status = StartupStatus::RuntimeActivated;
        self.install(namespace, paths, next)
    }

    pub(crate) fn install(
        &mut self,
        namespace: &WorkerNamespace,
        paths: &Paths,
        next: StartupCheckpointEvidence,
    ) -> Result<()> {
        validation::validate(&next)?;
        let expected = self
            .predecessor_authority
            .as_ref()
            .map(|authority| authority.checkpoint_identity);
        let installed = io::replace(namespace, paths, &self.evidence, &next, expected)?;
        self.checkpoint_identity = installed;
        if let Some(authority) = self.predecessor_authority.as_mut() {
            authority.checkpoint_identity = installed;
        }
        self.evidence = next;
        Ok(())
    }

    pub(crate) fn predecessor_authority(&self) -> Option<&StartupPredecessorAuthority> {
        self.predecessor_authority.as_ref()
    }

    pub(crate) fn commit_runtime_predecessor_publication(&mut self) {
        self.predecessor_authority = None;
    }
}
