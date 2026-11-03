use super::intake_test_support::IntakeFixture;
use crate::agent::hooks::{EventSequence, FIRST_SEQUENCE, HookEnvelope};
use crate::codex_hooks::{
    CodexHookIntake, CodexIntakeOccurrence, CodexOccurrenceIdentity, CodexOutbox, CodexOutboxState,
};
use std::os::unix::fs::PermissionsExt;

const REPLACEMENT_FILE_NAME: &str = "receipt-replacement.json";
const NEW_OCCURRENCE_MISSING: &str = "new occurrence must be discoverable";

pub(super) struct ReceiptFixture {
    source: IntakeFixture,
    pub(super) intake: CodexHookIntake,
    pub(super) outbox: CodexOutbox,
}

pub(super) struct PersistedOccurrence {
    pub(super) raw: HookEnvelope,
    pub(super) assigned: HookEnvelope,
    pub(super) identity: CodexOccurrenceIdentity,
    pub(super) occurrence: CodexIntakeOccurrence,
}

impl ReceiptFixture {
    pub(super) fn new() -> Self {
        let source = IntakeFixture::new();
        let intake = source.intake();
        let outbox =
            CodexOutbox::for_identity(source.artifacts.outbox_path(), &source.identity).unwrap();
        Self {
            source,
            intake,
            outbox,
        }
    }

    pub(super) fn raw(&self) -> HookEnvelope {
        self.source.envelope()
    }

    pub(super) fn persist(&self) -> PersistedOccurrence {
        let occurrence = self.persist_unassigned();
        self.assign(occurrence)
    }

    pub(super) fn persist_unassigned(&self) -> CodexIntakeOccurrence {
        let raw = self.intake.persist_occurrence(self.raw()).unwrap();
        self.intake
            .find_exact_occurrence(&raw)
            .unwrap()
            .expect(NEW_OCCURRENCE_MISSING)
    }

    pub(super) fn assign(&self, occurrence: CodexIntakeOccurrence) -> PersistedOccurrence {
        let raw = occurrence.envelope().clone();
        let identity = occurrence.durable_identity().unwrap();
        let assigned = self
            .outbox
            .enqueue_admitted(
                raw.clone(),
                Some(&identity),
                |_, _| Ok(()),
                |_, _, _| Ok(()),
            )
            .unwrap()
            .into_parts()
            .0;
        PersistedOccurrence {
            raw,
            assigned,
            identity,
            occurrence,
        }
    }

    pub(super) fn acknowledge(&self, persisted: &PersistedOccurrence) -> anyhow::Result<()> {
        self.outbox
            .acknowledge_occurrence(&persisted.assigned, &persisted.raw, &persisted.identity)
    }

    pub(super) fn append_acknowledged(&self, count: usize) {
        for _ in 0..count {
            let mut raw = self.raw();
            raw.event_id = uuid::Uuid::new_v4();
            let assigned = self.outbox.enqueue(raw).unwrap();
            self.outbox
                .acknowledge(assigned.sequence.0, assigned.event_id)
                .unwrap();
        }
    }

    pub(super) fn state(&self) -> CodexOutboxState {
        self.outbox.load().unwrap()
    }

    pub(super) fn receipt_count(&self) -> usize {
        self.state().occurrence_receipts.len()
    }

    pub(super) fn seed_receipts(&self, count: usize) {
        let source = self.persist_unassigned();
        let source_identity = source.durable_identity().unwrap().file_identity();
        let acknowledged_sequence = u64::try_from(count).unwrap();
        let mut state = CodexOutboxState {
            acknowledged_sequence,
            next_sequence: acknowledged_sequence.checked_add(FIRST_SEQUENCE).unwrap(),
            owner: Some(self.source.identity.clone()),
            ..CodexOutboxState::default()
        };
        for offset in 0..count {
            let sequence = u64::try_from(offset)
                .unwrap()
                .checked_add(FIRST_SEQUENCE)
                .unwrap();
            let mut raw = self.raw();
            raw.event_id = uuid::Uuid::new_v4();
            let file_name = super::super::intake_record::pending_file_name(sequence, raw.event_id);
            let identity =
                CodexOccurrenceIdentity::capture(&file_name, source_identity, &raw).unwrap();
            let mut assigned = raw.clone();
            assigned.sequence = EventSequence(sequence);
            let receipt = super::super::outbox_receipt::CodexOccurrenceReceipt::capture(
                &assigned, &raw, &identity,
            )
            .unwrap();
            state.acknowledged_events.push_back(assigned);
            state.occurrence_receipts.push_back(receipt);
        }
        super::super::outbox_validation::validate(&state).unwrap();
        crate::paths::atomic_write_json(self.outbox.path(), &state).unwrap();
        assert_eq!(self.state(), state);
    }

    pub(super) fn replace_same_content(
        &self,
        persisted: &PersistedOccurrence,
    ) -> CodexIntakeOccurrence {
        let path = self.intake.path(persisted.identity.file_name());
        let replacement = path.parent().unwrap().join(REPLACEMENT_FILE_NAME);
        std::fs::write(&replacement, std::fs::read(&path).unwrap()).unwrap();
        std::fs::set_permissions(
            &replacement,
            std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
        )
        .unwrap();
        std::fs::rename(replacement, path).unwrap();
        self.intake
            .find_exact_occurrence(&persisted.raw)
            .unwrap()
            .unwrap()
    }
}
