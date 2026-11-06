use crate::{Mode, State, network::Job};
use tokio::sync::mpsc;
use yolo_builtin_client_contract::ClientCapability;
use yolo_hub_identity::HubId;
use yolo_presentation_layout::Axis;
use yolo_tui_tab::{LocalTabView, MAX_TABS, TabCatalog, TabViewMode};

const SINGLE_PANE: usize = 1;
const TAB_LIMIT: &str = "Close a tab before opening another";

#[derive(Clone, Copy)]
pub(crate) enum Placement<'a> {
    Tab,
    NamedTab(&'a str),
    Split(Axis),
    Empty,
}

pub(crate) fn create(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    origin: HubId,
    cwd: String,
    name: Option<String>,
    placement: Placement<'_>,
) -> bool {
    if !crate::client_capabilities::require(
        state,
        ClientCapability::TerminalLifecycle,
        crate::client_capabilities::TERMINAL_LIFECYCLE_UNAVAILABLE,
    ) {
        return false;
    }
    let tabs = match reserve(state, placement) {
        Ok(destination) => destination,
        Err(error) => {
            state.status = error.to_string();
            return false;
        }
    };
    let previous = crate::creation_reservation::capture(state);
    state.replace_tabs(tabs);
    let previous_mode = std::mem::replace(&mut state.mode, Mode::Terminal);
    // Capture the new destination in the durable request, but defer view teardown
    // until enqueue succeeds. Failed validation/storage/queue admission preserves
    // selection, zoom, pointer capture and all existing input/query authority.
    let Some(request) = crate::shell_actions::enqueue_shell(state, jobs, origin, cwd, name) else {
        state.replace_tabs(previous.tabs);
        state.mode = previous_mode;
        return false;
    };
    super::terminal_picker::close(state);
    state.activate_tab_collection();
    crate::creation_reservation::retain(state, request, previous);
    true
}

fn reserve(state: &State, placement: Placement<'_>) -> anyhow::Result<TabCatalog> {
    let empty = vacant(state);
    let mut tabs = state.tab_snapshot();
    let mut layout = state.tab_layout().clone();
    match placement {
        Placement::Empty | Placement::Tab | Placement::NamedTab(_) if !empty => {
            let name = match placement {
                Placement::NamedTab(name) => name.to_owned(),
                _ => tab_name(&tabs)?,
            };
            tabs.create(&name)?;
            layout = tabs.active().view.layout.clone();
        }
        Placement::Split(axis) if !empty => {
            layout.split(axis, None)?;
        }
        Placement::NamedTab(name) => tabs.rename(tabs.active().id, name)?,
        _ => {}
    }
    tabs.update_active(LocalTabView {
        layout: layout.clone(),
        view: TabViewMode::Terminal,
    });
    Ok(tabs)
}

fn vacant(state: &State) -> bool {
    let panes = state.tab_layout().panes();
    panes.len() == SINGLE_PANE
        && panes.iter().all(|pane| pane.terminal.is_none())
        && !state
            .creations
            .pending
            .values()
            .any(|pending| pending.binding_active && pending.tab == state.tabs.active().id)
}

pub(super) fn tab_name(tabs: &TabCatalog) -> anyhow::Result<String> {
    anyhow::ensure!(tabs.count_for(tabs.machine()) < MAX_TABS, TAB_LIMIT);
    tabs.next_name(tabs.machine())
}
