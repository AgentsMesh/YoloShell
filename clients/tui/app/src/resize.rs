use crate::{State, events, network::Job, query_state::same_writer};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{ResizeRequest, TerminalSize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PendingResize {
    pub request_id: Uuid,
    pub grant: yolo_hub_terminal_protocol::LeaseGrant,
    pub size: TerminalSize,
}

/// A resize or focus loss invalidates hit-test geometry. A release outside
/// the outer terminal must not complete a stale drag, and paint skipped while
/// a terminal holds synchronized output must not leave old rectangles behind.
pub(crate) fn invalidate_layout(state: &mut State, geometry_changed: bool) {
    state.pointer = crate::pointer::Pointer::default();
    crate::selection_actions::cancel(state);
    if geometry_changed {
        state.presentation = Default::default();
    }
}

pub(crate) fn apply(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    view: yolo_tui_render::PaneViewport,
) {
    let Some(address) = view.terminal else { return };
    let Some(grant) = state.grant(address, std::time::Instant::now()) else {
        return;
    };
    let Ok(size) = TerminalSize::new(view.inner.width, view.inner.height) else {
        return;
    };
    let in_flight = state.resize_in_flight(address);
    if in_flight.is_some_and(|pending| !same_writer(pending.grant, grant)) {
        // A replaced writer cannot inherit an old request fence.
        retire(state, address);
    }
    let in_flight = state.resize_in_flight(address);
    if in_flight.is_some() {
        // Keep only the latest geometry while the control request is in
        // flight. The next draw will admit it after this request completes.
        state.set_last_size(address, size);
        return;
    }
    if state.last_size(address) == Some(size) && !state.is_resize_pending(address) {
        return;
    }
    let request = ResizeRequest {
        request_id: Uuid::new_v4(),
        grant,
        size,
    };
    if !state.is_offline(address)
        && state.surfaces.get(&address).is_some_and(|surface| {
            crate::observation_events::current(state, surface.cursor().target)
        })
    {
        state.mark_geometry_syncing(address);
    }
    let accepted = events::enqueue(
        state,
        jobs,
        Job::Call {
            generation: None,
            origin: address.hub_id,
            command: Command::Resize { request },
        },
    );
    state.set_last_size(address, size);
    state.mark_resize_pending(address);
    if accepted {
        state.set_resize_in_flight(
            address,
            PendingResize {
                request_id: request.request_id,
                grant: request.grant,
                size,
            },
        );
    }
}

pub(crate) fn completed(state: &mut State, request: &ResizeRequest) {
    let address = request.grant.fields().target.address;
    if !writer_matches(state, request) || !request_matches(state, request) {
        return;
    }
    state.clear_resize_in_flight(address);
    if state.last_size(address) == Some(request.size) {
        state.clear_resize_pending(address);
        state.mark_resync(address);
    }
}

pub(crate) fn failed(state: &mut State, request: &ResizeRequest) {
    let address = request.grant.fields().target.address;
    if !writer_matches(state, request) || !request_matches(state, request) {
        return;
    }
    state.clear_resize_in_flight(address);
}

fn writer_matches(state: &State, request: &ResizeRequest) -> bool {
    state
        .writers
        .get(&request.grant.fields().target.address)
        .is_some_and(|writer| same_writer(writer.grant, request.grant))
}

fn request_matches(state: &State, request: &ResizeRequest) -> bool {
    state
        .resize_in_flight(request.grant.fields().target.address)
        .is_some_and(|pending| {
            pending.request_id == request.request_id
                && pending.grant == request.grant
                && pending.size == request.size
        })
}

pub(crate) fn retire(state: &mut State, address: yolo_hub_identity::TerminalAddress) {
    state.clear_writer_geometry_facts(address);
}

pub(crate) fn snapshot_applied(state: &mut State, address: yolo_hub_identity::TerminalAddress) {
    if !state.is_resize_pending(address)
        && !state.is_resync(address)
        && state.surfaces.get(&address).is_some_and(|surface| {
            state
                .last_size(address)
                .is_some_and(|size| surface.screen().size() == (size.rows(), size.cols()))
        })
    {
        state.clear_geometry_syncing(address);
    }
}
