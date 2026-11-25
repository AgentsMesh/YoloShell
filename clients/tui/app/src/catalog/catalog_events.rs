use crate::{State, constants::CONNECTED_LABEL, events::enqueue, network::Job};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_client_channels::CatalogEvent;

pub(crate) fn changes(command: &yolo_hub_rpc_protocol::Command) -> bool {
    use yolo_hub_rpc_protocol::Command;
    matches!(
        command,
        Command::Create { .. }
            | Command::Finalize { .. }
            | Command::Terminal { .. }
            | Command::Get { .. }
            | Command::Resize { .. }
    )
}

pub(crate) fn route_probe(state: &mut State, jobs: &mpsc::Sender<Job>, hub: yolo_hub_identity::HubId) { if state.inventory.machine_access(hub, state.entry_hub) == yolo_tui_inventory::MachineAccess::Unknown && crate::events::enqueue(state, jobs, Job::Call { generation: None, origin: state.entry_hub, command: yolo_hub_rpc_protocol::Command::GetRouteCapabilities { target: hub } }) { state.status = crate::constants::MACHINE_ACCESS_CHECKING_LABEL.into(); } }

pub fn refresh(state: &mut State, jobs: &mpsc::Sender<Job>) {
    if state.catalog_pending.is_some() {
        return;
    }
    let request_id = Uuid::new_v4();
    if enqueue(state, jobs, Job::Refresh { request_id }) {
        state.catalog_pending = Some(request_id);
    }
}

pub fn invalidate(state: &mut State, jobs: &mpsc::Sender<Job>) {
    state.catalog_pending = None;
    state.inventory.current = false; state.inventory.route_capabilities.clear();
    refresh(state, jobs);
}

pub fn apply(state: &mut State, event: CatalogEvent) {
    if state.catalog_pending != Some(event.request_id) {
        return;
    }
    state.catalog_pending = None;
    match event.result {
        Ok(catalog) => {
            if state
                .recoverable_status
                .take()
                .is_some_and(|error| state.status == error)
            {
                state.status = if state.control_generation.is_some() {
                    CONNECTED_LABEL.into()
                } else {
                    crate::constants::CONNECTING_LABEL.into()
                };
            }
            state.notifications.observe(
                &catalog.terminals,
                state.inventory.current,
                &mut state.inventory.attention,
                std::time::Instant::now(),
            );
            let coherent = catalog.is_coherent();
            state.inventory.replace(catalog.agents, catalog.hubs);
            state.inventory.current = coherent;
            state.replace_terminals(catalog.terminals);
        }
        Err(error) => {
            state.inventory.current = false;
            let message = error.to_string();
            state.recoverable_status = Some(message.clone());
            state.status = message;
        }
    }
}
