use crate::{State, creation_events, network::Job};
use tokio::sync::mpsc;
use yolo_hub_client_channels::{ControlError, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};

/// A creation's own correlated outcome may outlive the fast writer connection.
/// It can bind a view, but can never restore any writer capability.
pub(crate) fn apply(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    control: &ControlRequest,
    result: Result<Reply, ControlError>,
) {
    let Command::Create { request } = &control.command else {
        return;
    };
    if !matches(state, control) {
        return;
    }
    match result {
        Ok(Reply::Created { creation }) => creation_events::created_from_generation(
            state,
            jobs,
            request.clone(),
            creation,
            control.generation,
        ),
        result => {
            if state
                .creations
                .pending
                .get(&request.fields().request_id)
                .is_some_and(|pending| pending.confirmed.is_some())
            {
                return;
            }
            let present = state
                .creations
                .pending
                .get(&request.fields().request_id)
                .is_some_and(|pending| crate::creation_presentation::current(state, pending));
            if result
                .as_ref()
                .err()
                .is_some_and(crate::creation_rejection::proves_uncreated)
            {
                crate::creation_rejection::retire(state, request);
            } else {
                state.creations.retire(request);
            }
            if present {
                let message = match result {
                    Err(error) => error.to_string(),
                    Ok(_) => crate::constants::UNEXPECTED_REPLY_LABEL.into(),
                };
                crate::creation_rejection::present(state, message);
            }
        }
    }
}

pub(crate) fn matches(state: &State, control: &ControlRequest) -> bool {
    let Command::Create { request } = &control.command else {
        return false;
    };
    state
        .creations
        .pending
        .get(&request.fields().request_id)
        .is_some_and(|pending| {
            pending.request == *request
                && pending.origin == control.destination
                && pending.generation == control.generation
        })
}
