use super::constants::{INITIAL_AUTHORITY_REVISION, MAX_REPLAY_WINDOW};
use super::snapshot::revision::AuthorityRevision;
use super::{EventSequence, HookEnvelope, Reducer, ReducerSnapshot};

const EVENT_COUNT_TO_SPAN: usize = 1;

impl Reducer {
    pub fn exhaust_active_authority_for_test(&mut self) {
        self.revision = AuthorityRevision::last_usable();
    }

    pub fn exhausted_active_snapshot_for_test(self) -> (ReducerSnapshot, Vec<HookEnvelope>) {
        self.active_snapshot_at_revision_for_test(AuthorityRevision::last_usable())
    }

    pub fn penultimate_active_snapshot_for_test(self) -> (ReducerSnapshot, Vec<HookEnvelope>) {
        self.active_snapshot_at_revision_for_test(AuthorityRevision::penultimate())
    }

    pub const fn last_usable_authority_revision_for_test() -> u64 {
        AuthorityRevision::last_usable().get()
    }

    fn active_snapshot_at_revision_for_test(
        mut self,
        revision: AuthorityRevision,
    ) -> (ReducerSnapshot, Vec<HookEnvelope>) {
        assert_eq!(self.state.history.len(), MAX_REPLAY_WINDOW);
        let last_sequence = revision
            .get()
            .checked_sub(INITIAL_AUTHORITY_REVISION)
            .unwrap();
        let span = u64::try_from(MAX_REPLAY_WINDOW - EVENT_COUNT_TO_SPAN).unwrap();
        let first_sequence = last_sequence.checked_sub(span).unwrap();
        let retained: Vec<_> = std::mem::take(&mut self.state.history)
            .into_values()
            .collect();
        self.state.event_ids.clear();
        for (offset, mut event) in retained.into_iter().enumerate() {
            event.sequence = EventSequence(
                first_sequence
                    .checked_add(u64::try_from(offset).unwrap())
                    .unwrap(),
            );
            self.state.event_ids.insert(event.event_id, event.sequence);
            self.state.history.insert(event.sequence, event);
        }
        self.state.last_sequence = EventSequence(last_sequence);
        self.revision = revision;
        let events = self.state.history.values().cloned().collect();
        (self.snapshot().unwrap(), events)
    }
}
