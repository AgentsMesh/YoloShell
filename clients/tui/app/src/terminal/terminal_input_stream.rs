use crate::{State, query_state::same_writer};
use std::time::{Duration, Instant};
use uuid::Uuid;
use yolo_hub_client_channels::{ControlRequest, InputStream};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::LeaseGrant;

/// Completion responsibility and the serial failure fence belong to exact
/// writer authority; view queues cancel only their own not-yet-sent requests.
pub(crate) struct Stream {
    pub generation: Uuid,
    pub grant: LeaseGrant,
    pub queue: InputStream,
}

impl Stream {
    pub fn new(generation: Uuid, grant: LeaseGrant, expires: Instant) -> Self {
        Self {
            generation,
            grant,
            queue: InputStream::new(expires),
        }
    }

    pub fn same_authority(&self, generation: Option<Uuid>, grant: LeaseGrant) -> bool {
        generation == Some(self.generation) && same_writer(self.grant, grant)
    }

    pub fn matches(&self, request: &ControlRequest) -> bool {
        matches!(&request.command, Command::Input { request: input }
            if self.same_authority(request.generation, input.fields().grant))
    }

    fn current(&self, state: &State) -> bool {
        let target = self.grant.fields().target;
        self.queue.is_current()
            && state.control_generation == Some(self.generation)
            && state
                .grant(target.address, Instant::now())
                .is_some_and(|grant| same_writer(self.grant, grant))
    }
}

pub(crate) fn sync(state: &mut State) {
    let addresses = state.inputs.streams.keys().copied().collect::<Vec<_>>();
    for address in addresses {
        let stream = &state.inputs.streams[&address];
        let pending = state.inputs.pending_in_stream(stream);
        let has_view = state
            .inputs
            .scope
            .as_ref()
            .is_some_and(|scope| stream.same_authority(Some(scope.generation), scope.grant));
        let current = stream.current(state);
        let target = stream.grant.fields().target;
        if !crate::observation_events::current(state, target)
            || (!pending
                && !state.inputs.interrupted.contains_key(&address)
                && (!has_view || !current))
        {
            if has_view {
                state.inputs.scope = None;
            }
            state.inputs.streams.remove(&address);
            state.inputs.interrupted.remove(&address);
            state.inputs.resume_requested.remove(&address);
            state.inputs.failures.remove(&address);
        } else if !current && pending {
            let grant = stream.grant;
            let already_interrupted = state
                .inputs
                .interrupted
                .get(&address)
                .is_some_and(|previous| same_writer(*previous, grant));
            stream.queue.retire();
            state.inputs.interrupted.insert(address, grant);
            if !already_interrupted {
                state.inputs.resume_requested.remove(&address);
            }
        }
    }
}

/// A confirmed new writer after explicit recovery replaces the old obligation.
/// Renewal and automatic acquisition cannot bury an unresolved input prefix.
pub(crate) fn granted(state: &mut State, grant: LeaseGrant, received: Instant) {
    let address = grant.fields().target.address;
    let Some(previous) = state.inputs.streams.get(&address) else {
        return;
    };
    let Some(generation) = state.control_generation else {
        return;
    };
    let expires = received + Duration::from_millis(grant.fields().ttl_ms);
    if previous.same_authority(Some(generation), grant) {
        previous.queue.renew_authority(expires);
        return;
    }
    if previous.grant.fields().target != grant.fields().target
        || (!same_writer(previous.grant, grant) && state.inputs.resume_requested.contains(&address))
    {
        if state
            .inputs
            .scope
            .as_ref()
            .is_some_and(|scope| scope.grant.fields().target.address == address)
        {
            state.inputs.scope = None;
        }
        state
            .inputs
            .streams
            .insert(address, Stream::new(generation, grant, expires));
        state.inputs.interrupted.remove(&address);
        state.inputs.resume_requested.remove(&address);
        state.inputs.failures.remove(&address);
    } else if state.inputs.pending_in_stream(previous) {
        let previous = previous.grant;
        state.inputs.interrupted.insert(address, previous);
        state.inputs.resume_requested.remove(&address);
    } else if !state.inputs.interrupted.contains_key(&address) {
        state.inputs.streams.remove(&address);
    }
}
