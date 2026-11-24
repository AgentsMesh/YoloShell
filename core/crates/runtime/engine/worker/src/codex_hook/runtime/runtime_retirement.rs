use super::super::authority::{
    CommittedProcessExit, LifecycleAuthority, is_authority_exhausted_error,
};
use super::CodexHookRuntime;
use crate::agent::hooks::{ProcessIdentity, ProcessState, ReducerSnapshot};
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexRetirementRecord, CodexRetirementSealArtifact,
    CodexRetirementWriterFence, CodexSnapshotProof, CodexTerminalOutboxProof,
};
use anyhow::{Context, Result, ensure};

pub(in crate::worker::codex_hook) struct CodexPreparedRetirement {
    pub(in crate::worker::codex_hook) record: CodexRetirementRecord,
    pub(in crate::worker::codex_hook) snapshot: ReducerSnapshot,
    pub(in crate::worker::codex_hook) proof: CodexSnapshotProof,
    pub(in crate::worker::codex_hook) outbox: CodexTerminalOutboxProof,
}

impl CodexHookRuntime {
    pub(in crate::worker::codex_hook) fn prepare_retirement(
        &self,
        writer: &CodexRetirementWriterFence,
        seal: &CodexRetirementSealArtifact,
        admission: &CodexAdmissionAuthority,
    ) -> Result<CodexPreparedRetirement> {
        writer.verify_seal(seal, admission)?;
        ensure!(
            admission.record().request().identity == self.owner,
            super::super::constants::CODEX_OWNER_ERROR
        );
        let mut lifecycle = self.lock_lifecycle_anyhow()?;
        settle_lifecycle(
            &mut lifecycle,
            &self.outbox,
            seal.record().identity().process,
        )?;
        let snapshot = lifecycle
            .reducer()?
            .snapshot()
            .context(super::super::constants::CODEX_STORAGE_ERROR)?;
        self.outbox.settle_terminal(&snapshot, admission)?;
        let outbox = self.outbox.prove_terminal(&snapshot, admission)?;
        let proof = lifecycle.prove_snapshot_current()?;
        let record = CodexRetirementRecord::capture(seal, lifecycle.reducer()?, admission, &proof)?;
        self.publish_lifecycle(&proof, &lifecycle)?;
        writer.verify_seal(seal, admission)?;
        Ok(CodexPreparedRetirement {
            record,
            snapshot,
            proof,
            outbox,
        })
    }
}

fn settle_lifecycle(
    lifecycle: &mut LifecycleAuthority,
    outbox: &crate::codex_hooks::CodexOutbox,
    observed: ProcessIdentity,
) -> Result<()> {
    if lifecycle.authority_exhausted() {
        return Ok(());
    }
    let result = super::super::outbox_authority::with_locked_state(outbox, |state| {
        super::super::replay::reconcile(lifecycle, state)?;
        if terminal(lifecycle)? {
            return Ok(());
        }
        match lifecycle.process_exit_committed(observed)? {
            CommittedProcessExit::Accepted(_, _) => Ok(()),
            CommittedProcessExit::Rejected(_, _) => Err(anyhow::anyhow!(
                super::super::constants::CODEX_PROCESS_ERROR
            )),
        }
    });
    match result {
        Err(error) if is_authority_exhausted_error(&error) && lifecycle.authority_exhausted() => {
            Ok(())
        }
        outcome => outcome,
    }
}

fn terminal(lifecycle: &LifecycleAuthority) -> Result<bool> {
    let reducer = lifecycle.reducer()?;
    Ok(reducer.state().process == ProcessState::Exited
        || reducer.is_revoked()
        || reducer.authority_exhausted())
}
