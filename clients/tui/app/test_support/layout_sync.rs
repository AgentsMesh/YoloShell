use crate::{
    State,
    layout_sync::{Operation, Sync},
    network::Job,
};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_presentation_protocol::{LayoutAddress, LayoutUpdate, SharedLayoutDocument};

pub fn fixture() -> (State, mpsc::Sender<Job>, mpsc::Receiver<Job>) {
    let mut state = crate::client_capability_support::state();
    let tabs = state.tab_snapshot().shared_tabs(state.entry_hub).unwrap();
    let document = LayoutUpdate {
        address: LayoutAddress {
            hub_id: HubId::new(),
            layout_id: Uuid::new_v4(),
        },
        request_id: Uuid::new_v4(),
        expected_revision: 0,
        tabs: tabs.clone(),
    }
    .apply(None)
    .unwrap();
    state.shared_layout = Some(Sync::new(document, tabs));
    let (jobs, incoming) = mpsc::channel(1);
    (state, jobs, incoming)
}

pub fn request(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    incoming: &mut mpsc::Receiver<Job>,
) -> Operation {
    crate::layout_sync::pump(state, jobs);
    let Job::Layout(operation) = incoming.try_recv().unwrap() else {
        panic!()
    };
    operation
}

pub fn response(state: &mut State, operation: Operation, document: SharedLayoutDocument) {
    crate::layout_sync::complete(
        state,
        operation,
        Ok(Reply::Layout {
            document: Some(document),
        }),
    );
}

pub fn remote(state: &State) -> SharedLayoutDocument {
    let current = &state.shared_layout.as_ref().unwrap().document;
    LayoutUpdate {
        address: current.address,
        request_id: Uuid::new_v4(),
        expected_revision: current.revision,
        tabs: current.tabs.clone(),
    }
    .apply(Some(current))
    .unwrap()
}

pub fn applied(state: &State, operation: &Operation) -> SharedLayoutDocument {
    let Command::UpdateLayout { change } = &operation.command else {
        panic!()
    };
    change
        .apply(Some(&state.shared_layout.as_ref().unwrap().document))
        .unwrap()
}
