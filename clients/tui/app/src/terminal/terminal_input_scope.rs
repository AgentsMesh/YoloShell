use crate::{State, query_state::same_writer};
use std::time::Instant;
use uuid::Uuid;
use yolo_hub_client_channels::RequestScope;
use yolo_hub_terminal_protocol::LeaseGrant;

pub(crate) struct Scope {
    pub queue: RequestScope,
    pub generation: Uuid,
    pub grant: LeaseGrant,
    pane: Uuid,
}

impl Scope {
    pub fn new(state: &State, generation: Uuid, grant: LeaseGrant) -> Self {
        Self {
            queue: state.inputs.streams[&grant.fields().target.address]
                .queue
                .view(),
            generation,
            grant,
            pane: state.tab_layout().focused(),
        }
    }

    pub fn current(&self, state: &State) -> bool {
        let target = self.grant.fields().target;
        let initial_snapshot_pending = state.inputs.snapshot_pending.contains(&target.address);
        self.same_view(state)
            && state.control_generation == Some(self.generation)
            && crate::observation_events::current(state, target)
            && (state.keyboard_ready(target.address) || initial_snapshot_pending)
            && state
                .grant(target.address, Instant::now())
                .is_some_and(|grant| same_writer(grant, self.grant))
    }

    pub fn same_view(&self, state: &State) -> bool {
        state.mode == crate::Mode::Terminal
            && !state.help
            && state.hub_manager.is_none()
            && state.tab_manager.is_none()
            && state.tab_layout().focused() == self.pane
            && state.focused_terminal() == Some(self.grant.fields().target.address)
    }
}
