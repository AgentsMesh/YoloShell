use crate::{State, creation_binding, creation_constants::PENDING_CREATION_INVARIANT};
use std::time::Instant;
use uuid::Uuid;
use yolo_tui_creation_journal::PendingFinalization;

pub(crate) fn flush(
    state: &mut State,
    jobs: &tokio::sync::mpsc::Sender<crate::network::Job>,
    now: Instant,
) {
    if state.creations.persistence_error.is_some() {
        return;
    }
    let ready = state
        .creations
        .pending
        .iter()
        .filter_map(|(id, pending)| {
            (pending.confirmed.is_some()
                && pending
                    .confirmation_retry_at
                    .is_none_or(|retry| retry <= now))
            .then_some(*id)
        })
        .collect::<Vec<_>>();
    for id in ready {
        confirm(state, jobs, id, now);
        if state.creations.persistence_error.is_some() {
            break;
        }
    }
    crate::finalization_persistence::clear_resolved_error(state);
}

fn confirm(
    state: &mut State,
    jobs: &tokio::sync::mpsc::Sender<crate::network::Job>,
    id: Uuid,
    now: Instant,
) {
    let pending = &state.creations.pending[&id];
    let Some(creation) = pending.confirmed else {
        return;
    };
    let record = PendingFinalization {
        origin: pending.origin,
        request: pending.request.clone(),
        creation,
    };
    if let Some(journal) = &mut state.creations.journal {
        if journal.retain_confirmed(record.clone()).is_err() {
            state
                .creations
                .pending
                .get_mut(&id)
                .expect(PENDING_CREATION_INVARIANT)
                .confirmation_retry_at = Some(now + crate::finalization_state::RETRY_DELAY);
            crate::finalization_persistence::failed(
                state,
                crate::finalization_persistence::RETAIN_FAILED,
                now,
            );
            return;
        }
    }
    let pending = state
        .creations
        .pending
        .remove(&id)
        .expect(PENDING_CREATION_INVARIANT);
    state.creations.finalizing.insert(
        record.request.fields().creation_token,
        crate::finalization_state::Finalization {
            record,
            tab: Some(pending.tab),
            pane: Some(pending.pane),
            attempt: None,
            retry_at: now,
            rejection: None,
            acknowledged: false,
        },
    );
    let present = crate::creation_presentation::current(state, &pending);
    let bound = creation_binding::apply(state, &pending, creation.address);
    if bound {
        // Binding is the event that makes the vacant pane concrete. Adopt
        // admitted bytes even when presentation/control authority was lost, so
        // a later writer grant can deliver them if the operator is still here.
        crate::terminal_input::adopt_creation_pregrant(state, &pending, creation.address);
    }
    let initial =
        bound && present && crate::initial_control::arm(state, &pending, creation.address, now);
    if bound && present {
        state.selected = Some(creation.address);
        state.inventory.view = yolo_tui_inventory::View::Agents;
        state.filter.clear();
    }
    if initial {
        crate::initial_control::catalog(state, jobs);
    }
}
