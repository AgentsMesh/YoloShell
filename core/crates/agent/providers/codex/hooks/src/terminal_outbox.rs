use super::lock::OutboxLock;
use super::{
    CodexAdmissionAuthority, CodexInvocationAuthority, CodexOutbox, CodexOutboxState,
    outbox_storage, outbox_validation,
};
use crate::agent::hooks::{FIRST_SEQUENCE, ProcessState, HookProviderKind, Reducer, ReducerSnapshot};
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;

const TERMINAL_OUTBOX_CORRUPT: &str = "Codex terminal outbox authority is corrupt";

pub struct CodexTerminalOutboxProof {
    lock: OutboxLock,
    path: PathBuf,
    loaded: outbox_storage::LoadedOutbox,
    invocation: CodexInvocationAuthority,
    admission: CodexAdmissionAuthority,
}

impl CodexOutbox {
    pub fn settle_terminal(
        &self,
        snapshot: &ReducerSnapshot,
        admission: &CodexAdmissionAuthority,
    ) -> Result<()> {
        let invocation = self.invocation_authority(admission)?;
        let lock = OutboxLock::acquire(&self.directory, &self.path)?;
        let mut loaded = outbox_storage::load(&lock, &self.path)?;
        let reducer = terminal_reducer(snapshot, admission)?;
        validate_reconciled(&loaded.state, &reducer)?;
        if loaded.state.pending.is_empty() {
            return Ok(());
        }
        while let Some(event) = loaded.state.pending.pop_front() {
            loaded.state.acknowledged_sequence = event.sequence.0;
            loaded.state.acknowledged_events.push_back(event);
            outbox_validation::trim_acknowledged(&mut loaded.state);
        }
        outbox_storage::persist(&lock, &self.path, &loaded)?;
        ensure!(
            self.invocation_authority(admission)? == invocation,
            TERMINAL_OUTBOX_CORRUPT
        );
        Ok(())
    }

    pub fn prove_terminal(
        &self,
        snapshot: &ReducerSnapshot,
        admission: &CodexAdmissionAuthority,
    ) -> Result<CodexTerminalOutboxProof> {
        let invocation = self.invocation_authority(admission)?;
        let lock = OutboxLock::acquire(&self.directory, &self.path)?;
        let loaded = outbox_storage::load(&lock, &self.path)?;
        validate_drained(&loaded.state, &terminal_reducer(snapshot, admission)?)?;
        let proof = CodexTerminalOutboxProof {
            lock,
            path: self.path.clone(),
            loaded,
            invocation,
            admission: admission.clone(),
        };
        proof.verify(snapshot, admission)?;
        Ok(proof)
    }

    fn invocation_authority(
        &self,
        admission: &CodexAdmissionAuthority,
    ) -> Result<CodexInvocationAuthority> {
        admission.verify()?;
        let observed = CodexInvocationAuthority::capture(&self.directory)?;
        ensure!(
            observed == admission.invocation_authority()?,
            TERMINAL_OUTBOX_CORRUPT
        );
        Ok(observed)
    }
}

impl CodexTerminalOutboxProof {
    pub(super) fn verify(
        &self,
        snapshot: &ReducerSnapshot,
        admission: &CodexAdmissionAuthority,
    ) -> Result<()> {
        ensure!(
            self.admission.exactly_matches(admission),
            TERMINAL_OUTBOX_CORRUPT
        );
        ensure!(
            self.invocation == admission.invocation_authority()?,
            TERMINAL_OUTBOX_CORRUPT
        );
        outbox_storage::verify_unchanged(&self.lock, &self.path, &self.loaded)?;
        validate_drained(&self.loaded.state, &terminal_reducer(snapshot, admission)?)?;
        outbox_storage::verify_unchanged(&self.lock, &self.path, &self.loaded)
    }
}

fn terminal_reducer(
    snapshot: &ReducerSnapshot,
    admission: &CodexAdmissionAuthority,
) -> Result<Reducer> {
    Reducer::restore(
        snapshot.clone(),
        admission.record().request().identity.clone(),
        HookProviderKind::Codex,
        super::ACCEPTED_PROVIDER_VERSION,
    )
    .map_err(anyhow::Error::new)
    .context(TERMINAL_OUTBOX_CORRUPT)
}

fn validate_reconciled(state: &CodexOutboxState, reducer: &Reducer) -> Result<()> {
    ensure!(terminal(reducer), TERMINAL_OUTBOX_CORRUPT);
    let lifecycle = reducer.state();
    let tip = state
        .next_sequence
        .checked_sub(FIRST_SEQUENCE)
        .context(TERMINAL_OUTBOX_CORRUPT)?;
    ensure!(lifecycle.last_sequence.0 == tip, TERMINAL_OUTBOX_CORRUPT);
    for event in &state.pending {
        ensure!(
            lifecycle.history.get(&event.sequence) == Some(event),
            TERMINAL_OUTBOX_CORRUPT
        );
    }
    Ok(())
}

fn validate_drained(state: &CodexOutboxState, reducer: &Reducer) -> Result<()> {
    validate_reconciled(state, reducer)?;
    ensure!(state.pending.is_empty(), TERMINAL_OUTBOX_CORRUPT);
    ensure!(
        state.acknowledged_sequence == reducer.state().last_sequence.0,
        TERMINAL_OUTBOX_CORRUPT
    );
    Ok(())
}

fn terminal(reducer: &Reducer) -> bool {
    reducer.state().process == ProcessState::Exited
        || reducer.state().revoked
        || reducer.authority_exhausted()
}
