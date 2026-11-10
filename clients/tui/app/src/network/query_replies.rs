use crate::{
    State,
    network::Job,
    query_state::{ActiveQueryScope, PendingReply, same_writer},
};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_client_channels::{ControlError, ControlRequest, RequestScope};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{InputFields, InputRequest, TerminalEvent};
use yolo_tui_terminal::QueryScope;

const MAX_PENDING_REPLIES: usize = crate::constants::QUEUE_CAPACITY;

pub(crate) fn live(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    generation: Uuid,
    event: TerminalEvent,
) {
    state.sync_query_authority();
    let target = event.target();
    let address = target.address;
    if event.is_terminal() {
        state.retire_query_scope(address);
        crate::observation_events::terminal(state, event);
        return;
    }
    let Some(grant) = state.query_grant(target) else {
        state.retire_query_scope(address);
        crate::observation_events::terminal(state, event);
        return;
    };
    let Some(control) = state.control_generation else {
        return;
    };
    let entry = state
        .queries
        .scopes
        .entry(address)
        .or_insert_with(|| ActiveQueryScope {
            parser: QueryScope::new(Uuid::new_v4()).unwrap(),
            queue: RequestScope::default(),
            grant,
            observer: generation,
            control,
        });
    let scope = entry.parser;
    let result = state
        .surfaces
        .get_mut(&address)
        .map(|surface| surface.apply_live_event(&event, Some(scope)));
    let replies = match result {
        Some(Ok(replies)) => replies,
        Some(Err(error)) => {
            state.surface_gap(address, Some(error.to_string()));
            return;
        }
        None => return,
    };
    // Replies are generated for this source session, independently of keyboard focus.
    for data in replies {
        if state.queries.pending.len() >= MAX_PENDING_REPLIES {
            break;
        }
        let Some(current) = state
            .query_grant(target)
            .filter(|current| same_writer(*current, grant))
        else {
            break;
        };
        let request_id = Uuid::new_v4();
        let Ok(request) = InputRequest::new(InputFields {
            request_id,
            grant: current,
            data,
        }) else {
            continue;
        };
        let Some(active) = state.queries.scopes.get(&address) else {
            break;
        };
        let expires = state.writers[&address].expires;
        let validity = active.queue.validity_until(expires);
        let job = Job::TerminalReply {
            generation: control,
            request: request.clone(),
            validity: validity.clone(),
        };
        if jobs.try_send(job).is_err() {
            break;
        }
        state.queries.pending.insert(
            request_id,
            PendingReply {
                request,
                generation: control,
                validity,
            },
        );
    }
}

pub(crate) fn completed(
    state: &mut State,
    request: &ControlRequest,
    result: Option<&Result<Reply, ControlError>>,
) -> Option<bool> {
    let Command::Input { request: input } = &request.command else {
        return None;
    };
    let id = input.fields().request_id;
    let expected = state.queries.pending.get(&id)?;
    if !expected.matches(request) {
        return Some(false);
    }
    state.queries.pending.remove(&id);
    // Rejected local queue admission is known not to have reached the origin.
    // An old scope's failed reply cannot revoke a replacement writer.
    if result.is_some_and(|result| {
        !matches!(
            result,
            Ok(Reply::Input { .. }) | Err(ControlError::Invalidated)
        )
    }) {
        let grant = input.fields().grant;
        let address = grant.fields().target.address;
        if state
            .writers
            .get(&address)
            .is_some_and(|writer| same_writer(writer.grant, grant))
        {
            state.retire_writer(address);
        }
    }
    Some(true)
}
