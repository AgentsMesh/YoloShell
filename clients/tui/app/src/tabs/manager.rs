use crate::{Mode, State, network::Job};
use crossterm::event::Event;
use tokio::sync::mpsc;
use yolo_hub_identity::HubId;
use yolo_tui_tab_manager::{Action, Manager};

pub(crate) fn handle(state: &mut State, jobs: &mpsc::Sender<Job>, hub: HubId, event: Event) {
    let Some(mut manager) = state.tab_manager.take() else {
        return;
    };
    let mut tabs = state.tab_snapshot();
    let Some(action) = manager.handle(event, &tabs) else {
        state.tab_manager = Some(manager);
        return;
    };
    if let Action::Create(name) = action {
        // Validate in a disposable catalog before admitting a real shell request.
        // Rejected names must never create a hidden PTY or change the active view.
        if let Err(error) = tabs.create(&name) {
            manager.error(error.to_string());
            state.tab_manager = Some(manager);
            return;
        }
        create(state, jobs, hub, manager, &name);
        return;
    }
    let old = tabs.active().id;
    let selecting = matches!(action, Action::Select(_));
    if let Action::Remove(id) = action {
        if let Err(error) = super::close::tab(state, id) {
            manager.error(error.to_string());
            state.tab_manager = Some(manager);
        }
        return;
    }
    let result = match action {
        Action::Select(id) => tabs.select(id).map(|_| ()),
        Action::Rename(id, name) => tabs.rename(id, &name),
        Action::Close | Action::Create(_) | Action::Remove(_) => return,
    };
    if let Err(error) = result {
        manager.error(error.to_string());
        state.tab_manager = Some(manager);
        return;
    }
    state.replace_tabs(tabs);
    if state.tabs.active().id != old {
        state.activate_tab_collection();
    }
    if selecting {
        super::terminal_picker::close(state);
        state.mode = Mode::Terminal;
        crate::polling::acquire(state, jobs, false);
    }
    state.sync_query_authority();
}

fn create(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    hub: HubId,
    mut manager: Manager,
    name: &str,
) {
    let origin = crate::shell_actions::destination(state, hub);
    let cwd = crate::shell_actions::directory(state, origin);
    if !super::creation::create(
        state,
        jobs,
        origin,
        cwd,
        None,
        super::creation::Placement::NamedTab(name),
    ) {
        manager.error(state.status.clone());
        state.tab_manager = Some(manager);
    }
}
