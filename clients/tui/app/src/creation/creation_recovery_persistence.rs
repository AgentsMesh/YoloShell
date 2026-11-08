use crate::{
    State, creation_constants::PENDING_CREATION_INVARIANT, finalization_state::Finalization,
};
use std::time::Instant;
use yolo_hub_terminal_protocol::CreationQueryOutcome;
use yolo_tui_creation_journal::{CreationIntent, PendingFinalization};

const PREPARE_FAILED: &str =
    "Cannot save the shell request; nothing was sent. New shell creation is paused";
const RESOLUTION_FAILED: &str =
    "Cannot save the recovered shell result; new shell creation is paused";
const UNCREATED_FAILED: &str =
    "Cannot retire the failed shell request; new shell creation is paused";
const ALREADY_PREPARED: &str = "This shell request is already being recovered";

pub(crate) fn prepare(state: &mut State, intent: CreationIntent, now: Instant) -> bool {
    let result = match &mut state.creations.journal {
        Some(journal) => journal.retain_intent(intent.clone()),
        None => Ok(true),
    };
    match result {
        Ok(true) => true,
        Ok(false) => {
            state
                .creations
                .recovery
                .retain(intent, now)
                .expect(PENDING_CREATION_INVARIANT);
            state.status = ALREADY_PREPARED.into();
            false
        }
        Err(_) => {
            // The caller still owns its unused queue permit. This fact is not
            // serialized and can never be inferred from a restarted journal.
            state
                .creations
                .uncreated
                .insert(intent.request.fields().creation_token, intent);
            crate::finalization_persistence::failed(state, PREPARE_FAILED, now);
            state.status = PREPARE_FAILED.into();
            false
        }
    }
}

pub(crate) fn flush(state: &mut State, now: Instant) {
    if state.creations.persistence_error.is_some() {
        return;
    }
    let uncreated = state
        .creations
        .uncreated
        .values()
        .cloned()
        .collect::<Vec<_>>();
    for intent in uncreated {
        if let Some(journal) = &mut state.creations.journal {
            if journal.cancel_uncreated_intent(&intent).is_err() {
                crate::finalization_persistence::failed(state, UNCREATED_FAILED, now);
                return;
            }
        }
        state
            .creations
            .uncreated
            .remove(&intent.request.fields().creation_token);
    }
    for (intent, query) in state.creations.recovery.resolutions() {
        let ready = match query.outcome() {
            CreationQueryOutcome::Ready { creation } => Some(PendingFinalization {
                origin: intent.origin,
                request: intent.request.clone(),
                creation: *creation,
            }),
            CreationQueryOutcome::Finalized { .. }
            | CreationQueryOutcome::Survivor { .. }
            | CreationQueryOutcome::Archived { .. } => None,
            // Only terminal evidence, or a Ready result that can be durably
            // promoted, permits resolution. Never retire Missing/Pending/Unknown.
            _ => continue,
        };
        let stored = match &mut state.creations.journal {
            Some(journal) => match &ready {
                Some(record) => journal.retain_confirmed(record.clone()),
                None => journal.settle_intent(&intent),
            },
            None => Ok(true),
        };
        if stored.is_err() {
            crate::finalization_persistence::failed(state, RESOLUTION_FAILED, now);
            return;
        }
        if let Some(record) = ready {
            state.creations.finalizing.insert(
                record.request.fields().creation_token,
                Finalization {
                    record,
                    tab: None,
                    pane: None,
                    attempt: None,
                    retry_at: now,
                    rejection: None,
                    acknowledged: false,
                },
            );
        }
        state
            .creations
            .recovery
            .settled(&intent)
            .expect(PENDING_CREATION_INVARIANT);
    }
    crate::finalization_persistence::clear_resolved_error(state);
}
