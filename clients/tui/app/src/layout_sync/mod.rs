mod completion;
mod constants;
pub(crate) mod network;
pub(crate) mod startup;

use crate::{State, network::Job};
pub(crate) use completion::complete;
pub(crate) use constants::QUEUE_CAPACITY;
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_rpc_protocol::Command;
use yolo_presentation_protocol::{LayoutUpdate, SharedLayoutDocument, SharedTab};

#[derive(Clone, Debug)]
pub(crate) struct Operation {
    pub token: Uuid,
    pub command: Command,
}

/// One serialized CAS lane. Uncertain writes are resolved by reading, never replayed.
pub(crate) struct Sync {
    pub document: SharedLayoutDocument,
    pub projected: Vec<SharedTab>,
    pub pending: Option<Operation>,
    pub uncertain: Option<LayoutUpdate>,
    pub read: bool,
    pub retry: Instant,
}

impl Sync {
    pub fn new(document: SharedLayoutDocument, projected: Vec<SharedTab>) -> Self {
        Self {
            document,
            projected,
            pending: None,
            uncertain: None,
            read: false,
            retry: Instant::now(),
        }
    }
}

pub(crate) fn refresh(state: &mut State) {
    if let Some(sync) = &mut state.shared_layout {
        sync.read = true;
    }
}

pub(crate) fn pump(state: &mut State, jobs: &mpsc::Sender<Job>) {
    let Ok(tabs) = state.tab_snapshot().shared_tabs(state.entry_hub) else {
        return;
    };
    let Some(sync) = &mut state.shared_layout else {
        return;
    };
    if sync.pending.is_some() || Instant::now() < sync.retry {
        return;
    }
    let command = if sync.read || sync.uncertain.is_some() {
        Command::GetLayout {
            address: sync.document.address,
        }
    } else if tabs != sync.projected {
        Command::UpdateLayout {
            change: LayoutUpdate {
                address: sync.document.address,
                request_id: Uuid::new_v4(),
                expected_revision: sync.document.revision,
                tabs,
            },
        }
    } else {
        return;
    };
    let operation = Operation {
        token: Uuid::new_v4(),
        command,
    };
    if jobs.try_send(Job::Layout(operation.clone())).is_ok() {
        sync.pending = Some(operation);
    }
}

pub(super) fn adopt(
    state: &mut State,
    document: &SharedLayoutDocument,
) -> anyhow::Result<Vec<SharedTab>> {
    let previous = state.tab_snapshot();
    let previous_active = previous.active().id;
    let previous_focus = previous.active().view.layout.focused();
    let previous_terminal = previous
        .active()
        .view
        .layout
        .panes()
        .into_iter()
        .find(|pane| pane.id == previous_focus)
        .and_then(|pane| pane.terminal);
    let tabs = yolo_tui_tab::TabCatalog::from_shared(document, &previous, state.entry_hub)?;
    let changed = tabs != previous;
    // Admission also runs when the remote topology is unchanged: the local
    // catalog may already hold a binding this client now knows is gone.
    let admission = state.replace_tabs(tabs);
    if changed || admission.retired {
        let layout = state.tab_layout().clone();
        let terminal = layout
            .panes()
            .into_iter()
            .find(|pane| pane.id == layout.focused())
            .and_then(|pane| pane.terminal);
        let same_focus = state.tabs.active().id == previous_active
            && layout.focused() == previous_focus
            && terminal == previous_terminal;
        if same_focus {
            state.replace_active_tab_layout(layout);
            state.pointer = Default::default();
            state.presentation = Default::default();
            state.sync_query_authority();
        } else {
            state.activate_tab_collection();
        }
    }
    // An empty document is a local placeholder, not a request to publish one.
    // For every non-empty document, `projected` is the server view so a pruned
    // local topology is published in the same round.
    if document.tabs.is_empty() {
        return state.tab_snapshot().shared_tabs(state.entry_hub);
    }
    Ok(document.tabs.clone())
}
