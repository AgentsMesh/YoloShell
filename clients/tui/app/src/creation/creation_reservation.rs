use crate::{Mode, State, creation_presentation::CreationPresentation};
use uuid::Uuid;
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::CreateShellRequest;
use yolo_tui_tab::TabCatalog;

pub(crate) struct Before {
    pub tabs: TabCatalog,
    directory: Option<DirectoryForm>,
}

pub(crate) struct Reservation {
    previous: Before,
    reserved: TabCatalog,
}

struct DirectoryForm {
    presentation: CreationPresentation,
    picker: Option<crate::tabs::terminal_picker::TerminalPicker>,
    selected: Option<TerminalAddress>,
    zoomed: bool,
}

pub(crate) fn capture(state: &State) -> Before {
    Before {
        tabs: state.tab_snapshot(),
        directory: (state.mode == Mode::Directory).then(|| DirectoryForm {
            presentation: CreationPresentation::capture(state),
            picker: state.terminal_picker.clone(),
            selected: state.selected,
            zoomed: state.pane_view.zoomed,
        }),
    }
}

pub(crate) fn retain(state: &mut State, request: Uuid, previous: Before) {
    let reserved = state.tab_snapshot();
    let presentation = CreationPresentation::capture(state);
    let pending = state
        .creations
        .pending
        .get_mut(&request)
        .expect(crate::creation_constants::PENDING_CREATION_INVARIANT);
    pending.presentation = presentation;
    pending.reservation = Some(Reservation { previous, reserved });
}

/// Only the exact unchanged provisional view can be undone after a proven no-resource outcome.
pub(crate) fn rollback(state: &mut State, request: &CreateShellRequest) {
    let id = request.fields().request_id;
    let current = state.creations.pending.get(&id).is_some_and(|pending| {
        pending.request == *request
            && pending.confirmed.is_none()
            && crate::creation_presentation::current(state, pending)
            && pending
                .reservation
                .as_ref()
                .is_some_and(|reservation| state.tab_snapshot() == reservation.reserved)
    });
    if !current {
        return;
    }
    let reservation = state
        .creations
        .pending
        .get_mut(&id)
        .and_then(|pending| pending.reservation.take())
        .expect(crate::creation_constants::PENDING_CREATION_INVARIANT);
    state.replace_tabs(reservation.previous.tabs);
    state.activate_tab_collection();
    if let Some(directory) = reservation.previous.directory {
        directory.presentation.restore_directory(state);
        state.terminal_picker = directory.picker;
        state.selected = directory.selected;
        state.pane_view.zoomed = directory.zoomed;
        state.entry = request.fields().cwd.clone();
        crate::form_input::reset(state);
    }
    state.sync_query_authority();
}
