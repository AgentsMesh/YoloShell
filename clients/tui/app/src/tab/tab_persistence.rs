use crate::{Mode, Options, State};
use anyhow::Result;
use std::time::Instant;
use yolo_tui_tab::{LayoutCheckpoint, TabStore, TabViewMode, open_client};

#[derive(Default)]
pub struct SaveState {
    last_verified: Option<Instant>,
    saved_tab_revision: Option<u64>,
    saved_view: Option<TabViewMode>,
}

pub fn open(options: &Options) -> Result<TabStore> {
    open_client(options.hub)
}

pub fn restore(store: &TabStore) -> State {
    let tab = &store.document().tabs.active().view;
    let mut state = State::default();
    state.replace_tabs(store.document().tabs.clone());
    state.selected = state.focused_terminal();
    let document = state.document.restore(&state.tabs);
    state.mode = if document {
        Mode::Document
    } else if tab.view == TabViewMode::Terminal {
        Mode::Terminal
    } else {
        Mode::Inventory
    };
    state
}

pub fn save(store: &mut TabStore, state: &State) -> Result<()> {
    let shared = shared_layout(state);
    store.save_presentation(&state.tab_snapshot(), shared)?;
    Ok(())
}

pub fn save_if_changed(store: &mut TabStore, state: &State, saved: &mut SaveState) -> Result<()> {
    save_if_changed_at(store, state, saved, Instant::now())
}

pub(crate) fn save_if_changed_at(
    store: &mut TabStore,
    state: &State,
    saved: &mut SaveState,
    now: Instant,
) -> Result<()> {
    if state.pointer.drag.is_some() {
        return Ok(());
    }
    let view = if matches!(state.mode, Mode::Terminal | Mode::Chat) {
        TabViewMode::Terminal
    } else {
        TabViewMode::Inventory
    };
    let tab_changed =
        saved.saved_tab_revision != Some(state.tab_revision) || saved.saved_view != Some(view);
    if !tab_changed
        && saved.last_verified.is_some_and(|verified| {
            now.saturating_duration_since(verified) < crate::constants::TAB_VERIFY_INTERVAL
        })
    {
        return Ok(());
    }
    if !tab_changed {
        store.verify_current()?;
        saved.last_verified = Some(now);
        saved.saved_view = Some(view);
        return Ok(());
    }
    let tabs = state.tab_snapshot();
    let shared = shared_layout(state);
    let unchanged = store.document().tabs == tabs && store.document().shared == shared;
    if unchanged {
        store.verify_current()?;
        saved.last_verified = Some(now);
        saved.saved_tab_revision = Some(state.tab_revision);
        saved.saved_view = Some(view);
        return Ok(());
    }
    store.save_presentation(&tabs, shared)?;
    saved.last_verified = Some(now);
    saved.saved_tab_revision = Some(state.tab_revision);
    saved.saved_view = Some(view);
    Ok(())
}

fn shared_layout(state: &State) -> Option<LayoutCheckpoint> {
    state
        .shared_layout
        .as_ref()
        .map(|sync| yolo_tui_tab::LayoutCheckpoint {
            document: sync.document.clone(),
            projected: sync.projected.clone(),
        })
}
