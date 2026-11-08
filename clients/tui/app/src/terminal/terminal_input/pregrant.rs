use crate::{State, network::Job};
use std::collections::{BTreeSet, VecDeque};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::{InputRequest, LeaseGrant};

/// Where admitted bytes belong while their terminal is being established.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PregrantTarget {
    Address(TerminalAddress),
    Creation(Uuid),
    Startup,
}

/// Keyboard bytes admitted while the exact destination writer is in flight.
pub(crate) struct PregrantInput {
    pub target: PregrantTarget,
    /// Bound to one connection as soon as the destination is concrete. Startup
    /// bytes are bound when their attach or creation resolves.
    pub generation: Option<Uuid>,
    pub data: Vec<u8>,
}

pub(crate) fn enqueue_pregrant(state: &mut State, address: TerminalAddress, data: Vec<u8>) -> bool {
    prune_pregrant(state);
    let Some(generation) = state
        .control_generation
        .filter(|_| state.focused_terminal() == Some(address))
    else {
        return false;
    };
    if !push_pregrant(
        state,
        PregrantTarget::Address(address),
        Some(generation),
        data,
    ) {
        state.inputs.overflowed.insert(address);
        return false;
    }
    true
}

/// Bytes typed into a new shell's vacant pane before the creation result names
/// its address.
pub(crate) fn enqueue_creation_pregrant(state: &mut State, data: Vec<u8>) -> bool {
    prune_pregrant(state);
    let Some(generation) = state.control_generation else {
        return false;
    };
    let Some(request_id) = current_creation(state) else {
        return false;
    };
    push_pregrant(
        state,
        PregrantTarget::Creation(request_id),
        Some(generation),
        data,
    )
}

/// Bytes typed while an explicit `attach` or `new` startup intent is waiting
/// for the catalogue/control handshake.
pub(crate) fn enqueue_startup_pregrant(state: &mut State, data: Vec<u8>) -> bool {
    prune_pregrant(state);
    push_pregrant(state, PregrantTarget::Startup, None, data)
}

/// Adjacent bytes for one destination form one contiguous admission batch, so
/// a fast command is not rejected by the per-batch capacity.
fn push_pregrant(
    state: &mut State,
    target: PregrantTarget,
    generation: Option<Uuid>,
    data: Vec<u8>,
) -> bool {
    if let Some(last) = state.inputs.pregrant.back_mut()
        && last.target == target
        && last.generation == generation
        && last.data.len() + data.len() <= yolo_hub_terminal_protocol::MAX_INPUT_BYTES
    {
        last.data.extend_from_slice(&data);
        return true;
    }
    if state.inputs.pregrant.len() >= crate::constants::PREGRANT_INPUT_CAPACITY {
        state.status = crate::constants::INPUT_OVERFLOW_LABEL.into();
        return false;
    }
    state.inputs.pregrant.push_back(PregrantInput {
        target,
        generation,
        data,
    });
    true
}

/// The startup attach resolved to an address; bind its bytes to this connection.
pub(crate) fn adopt_startup_pregrant(state: &mut State, address: TerminalAddress) -> bool {
    let Some(generation) = state.control_generation else {
        return false;
    };
    retarget(
        state,
        PregrantTarget::Startup,
        PregrantTarget::Address(address),
        Some(generation),
    )
}

/// The startup `new` resolved to a pending creation; keep the bytes tied to it.
pub(crate) fn adopt_startup_creation(state: &mut State) {
    let Some(generation) = state.control_generation else {
        return;
    };
    let Some(request_id) = current_creation(state) else {
        return;
    };
    retarget(
        state,
        PregrantTarget::Startup,
        PregrantTarget::Creation(request_id),
        Some(generation),
    );
}

/// A creation was bound to an address; its bytes now wait for the initial grant.
pub(crate) fn adopt_creation_pregrant(
    state: &mut State,
    pending: &crate::creation_state::PendingCreation,
    address: TerminalAddress,
) {
    let generation = state.control_generation;
    retarget(
        state,
        PregrantTarget::Creation(pending.request.fields().request_id),
        PregrantTarget::Address(address),
        generation,
    );
}

fn retarget(
    state: &mut State,
    from: PregrantTarget,
    to: PregrantTarget,
    generation: Option<Uuid>,
) -> bool {
    let mut found = false;
    for item in &mut state.inputs.pregrant {
        if item.target == from {
            item.target = to;
            item.generation = generation;
            found = true;
        }
    }
    found
}

include!("pregrant_lifecycle.rs");
