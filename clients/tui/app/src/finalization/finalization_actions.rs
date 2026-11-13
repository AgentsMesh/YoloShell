use crate::{
    State,
    finalization_state::{Attempt, RETRY_DELAY},
    network::Job,
};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_client_channels::RequestScope;

pub(crate) fn flush(state: &mut State, jobs: &mpsc::Sender<Job>, now: Instant) {
    crate::finalization_persistence::acknowledge(state, now);
    if state.creations.persistence_error.is_some()
        || !state.client_capability_enabled(ClientCapability::TerminalLifecycle)
    {
        return;
    }
    let Some(generation) = state
        .control_generation
        .filter(|generation| !generation.is_nil())
    else {
        return;
    };
    for (token, entry) in &mut state.creations.finalizing {
        if entry
            .attempt
            .as_ref()
            .is_some_and(|attempt| attempt.validity.deadline() <= now)
        {
            // A cancelled/panicked channel owner may never send Finished. Only
            // Finalize is safe to retry when its bounded attempt has expired.
            entry.retry(now);
        }
        if entry.acknowledged
            || entry.attempt.is_some()
            || entry.rejection.is_some()
            || entry.retry_at > now
        {
            continue;
        }
        let command = entry.command(*token);
        let scope = RequestScope::default();
        let validity =
            scope.validity_until(now + command.operation_timeout(crate::constants::RPC_TIMEOUT));
        let job = Job::Finalize {
            generation,
            creation_token: *token,
            creation: entry.record.creation,
            validity: validity.clone(),
        };
        if jobs.try_send(job).is_err() {
            entry.retry_at = now + RETRY_DELAY;
            break;
        }
        entry.attempt = Some(Attempt {
            generation,
            validity,
            scope,
        });
    }
}
