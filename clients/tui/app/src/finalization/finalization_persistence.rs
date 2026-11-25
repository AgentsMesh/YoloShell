use crate::{
    State,
    finalization_state::{Finalization, RETRY_DELAY},
};
use std::time::Instant;
use yolo_hub_identity::HubId;
use yolo_tui_creation_journal::CreationJournal;
use yolo_tui_tab::TabStore;

pub(crate) const RETAIN_FAILED: &str =
    "Cannot save confirmed shell creation; keep this TUI open. New shell creation is paused";
const ACKNOWLEDGE_FAILED: &str =
    "Cannot save shell finalization acknowledgement; new shell creation is paused";
const RECOVERY_FAILED: &str = "Terminal client storage is unavailable or changed; keep this TUI open and restore its original storage";
pub(crate) const RECOVER_STORAGE_KEY: char = 'f';

pub(crate) fn open(state: &mut State, tab: &TabStore, hub: HubId) -> anyhow::Result<()> {
    let journal = CreationJournal::open(tab.directory()?, hub)?;
    let now = Instant::now();
    for intent in journal.intents()? {
        state.creations.recovery.retain(intent, now)?;
    }
    for record in journal.pending()? {
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
    state.creations.journal = Some(journal);
    Ok(())
}

pub(crate) fn acknowledge(state: &mut State, now: Instant) {
    if state.creations.persistence_error.is_some() {
        return;
    }
    let ready = state
        .creations
        .finalizing
        .iter()
        .filter_map(|(token, entry)| {
            (entry.acknowledged && entry.retry_at <= now).then_some(*token)
        })
        .collect::<Vec<_>>();
    for token in ready {
        let entry = &state.creations.finalizing[&token];
        if let Some(journal) = &mut state.creations.journal {
            if journal.acknowledge(&entry.record).is_err() {
                state
                    .creations
                    .finalizing
                    .get_mut(&token)
                    .expect(crate::creation_constants::PENDING_CREATION_INVARIANT)
                    .retry_at = now + RETRY_DELAY;
                failed(state, ACKNOWLEDGE_FAILED, now);
                break;
            }
        }
        state.creations.finalizing.remove(&token);
    }
    clear_resolved_error(state);
}

pub(crate) fn clear_resolved_error(state: &mut State) {
    if state.creations.uncreated.is_empty()
        && state.creations.recovery.status().resolved == 0
        && !state
            .creations
            .pending
            .values()
            .any(|pending| pending.confirmed.is_some())
        && !state
            .creations
            .finalizing
            .values()
            .any(|entry| entry.acknowledged)
    {
        state.creations.persistence_error = None;
        state.creations.persistence_recover_at = None;
    }
}

pub(crate) fn failed(state: &mut State, message: &'static str, now: Instant) {
    state.creations.persistence_error = Some(message);
    state.creations.persistence_recover_at = Some(now + RETRY_DELAY);
}

pub(crate) fn recover_due(state: &mut State, now: Instant) {
    if !state
        .creations
        .persistence_recover_at
        .is_some_and(|at| at <= now)
    {
        return;
    }
    state.creations.persistence_recover_at = None;
    if let Some(journal) = &mut state.creations.journal {
        if journal.recover().is_err() {
            // Stop automatic attempts after proof-preserving recovery fails.
            // An explicit retry can recheck restored original storage; no reopen
            // or replacement-directory adoption is permitted here.
            state.creations.persistence_error = Some(RECOVERY_FAILED);
            return;
        }
    }
    state.creations.persistence_error = None;
}

pub(crate) fn request_recovery(state: &mut State, now: Instant) {
    if state.creations.persistence_error.is_some() {
        state.creations.persistence_recover_at = Some(now);
        recover_due(state, now);
    }
    state.creations.recovery.retry_blocked(now);
    crate::creation_recovery_persistence::flush(state, now);
}

pub(crate) fn status(state: &State) -> Option<String> {
    state
        .creations
        .persistence_error
        .map(|error| format!("{error} · Ctrl+B {RECOVER_STORAGE_KEY}: retry storage"))
}
