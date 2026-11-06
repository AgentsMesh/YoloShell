use super::retirement_test_support::RetirementFixture;
use crate::agent::hooks::{
    EventKind, EventSequence, ProcessExitDisposition, HookProviderKind, Reducer, ReducerSnapshot,
    SessionStartSource,
};
use crate::codex_hooks::{
    ACCEPTED_PROVIDER_VERSION, CodexOutbox, CodexRetirementRecord, CodexRetirementSealArtifact,
    CodexRetirementWriterFence, CodexSnapshotCursor, CodexSnapshotProof, CodexTerminalIntakeProof,
    CodexTerminalOutboxProof,
};

pub(super) struct TerminalFixture {
    pub(super) record: CodexRetirementRecord,
    pub(super) snapshot: ReducerSnapshot,
    pub(super) proof: CodexSnapshotProof,
    pub(super) intake: CodexTerminalIntakeProof,
    pub(super) outbox: CodexTerminalOutboxProof,
}

impl RetirementFixture {
    pub(super) fn terminal(
        &self,
        writer: &CodexRetirementWriterFence,
        seal: &CodexRetirementSealArtifact,
    ) -> TerminalFixture {
        let mut reducer = self.reducer();
        assert_eq!(reducer.process_exit(), ProcessExitDisposition::Unconfirmed);
        self.persist_terminal(writer, seal, reducer)
    }

    pub(super) fn alternate_terminal_snapshot(&self) -> ReducerSnapshot {
        let mut reducer = self.reducer();
        let mut start = self.envelope();
        start.event_id = uuid::Uuid::new_v4();
        start.sequence = EventSequence::FIRST;
        start.event = EventKind::SessionStart {
            source: SessionStartSource::Startup,
        };
        reducer.apply(start).unwrap();
        reducer.process_exit();
        reducer.snapshot().unwrap()
    }

    fn reducer(&self) -> Reducer {
        Reducer::new(
            self.admission.identity.clone(),
            HookProviderKind::Codex,
            ACCEPTED_PROVIDER_VERSION,
        )
        .unwrap()
    }

    fn persist_terminal(
        &self,
        writer: &CodexRetirementWriterFence,
        seal: &CodexRetirementSealArtifact,
        reducer: Reducer,
    ) -> TerminalFixture {
        let snapshot = reducer.snapshot().unwrap();
        let (cursor, loaded) = CodexSnapshotCursor::open(self.snapshot_path()).unwrap();
        assert!(loaded.is_none());
        drop(cursor.persist(&snapshot).unwrap());
        let outbox = CodexOutbox::for_identity(
            self.admission.artifacts.outbox_path(),
            &self.admission.identity,
        )
        .unwrap();
        outbox.settle_terminal(&snapshot, self.admission()).unwrap();
        let outbox = outbox.prove_terminal(&snapshot, self.admission()).unwrap();
        let (cursor, loaded) = CodexSnapshotCursor::open(self.snapshot_path()).unwrap();
        assert!(loaded.is_some());
        let proof = cursor.prove_current().unwrap();
        let intake = writer
            .prove_terminal_intake(seal, self.admission())
            .unwrap();
        let record =
            CodexRetirementRecord::capture(seal, &reducer, self.admission(), &proof).unwrap();
        TerminalFixture {
            record,
            snapshot,
            proof,
            intake,
            outbox,
        }
    }
}
