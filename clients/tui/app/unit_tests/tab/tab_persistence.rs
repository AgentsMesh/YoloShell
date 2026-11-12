use super::fixtures;
use crate::{Mode, tab_persistence};
use std::time::Instant;
use yolo_hub_identity::HubId;
use yolo_tui_tab::{LocalTabView, TabStore, TabViewMode};

#[test]
fn active_tab_layout_is_committed_before_the_tab_catalog_is_consumed() {
    let session = fixtures::session();
    let mut state = crate::client_capability_support::state();
    state.bind_focused(Some(session.fields().address)).unwrap();
    state.mode = Mode::Terminal;
    state.sync_active_tab_view();
    let active = &state.tabs.active().view;
    assert_eq!(active.layout, *state.tab_layout());
    assert_eq!(active.view, TabViewMode::Terminal);
}

#[test]
fn replacing_an_inactive_tab_catalog_cannot_overwrite_the_live_layout() {
    let mut state = crate::client_capability_support::state();
    let active_layout = state.tab_layout().clone();
    let mut catalog = state.tab_snapshot();
    catalog.create("Tab 2").unwrap();
    catalog.select(catalog.entries()[0].id).unwrap();
    state.replace_tabs(catalog);
    assert_eq!(*state.tab_layout(), active_layout);
}

#[test]
fn restored_tab_state_keeps_layout_but_never_restores_pending_commands_or_writers() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap();
    let hub = HubId::new();
    let mut store = TabStore::open(&path, hub).unwrap();
    let session = fixtures::session();
    let mut state = crate::client_capability_support::state();
    state.bind_focused(Some(session.fields().address)).unwrap();
    state.mode = Mode::Terminal;
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.record_grant(fixtures::grant(&session), std::time::Instant::now());
    tab_persistence::save(&mut store, &state).unwrap();
    let restored = tab_persistence::restore(&store);
    assert_eq!(*restored.tab_layout(), *state.tab_layout());
    assert_eq!(restored.mode, Mode::Terminal);
    assert_eq!(restored.focused_terminal(), Some(session.fields().address));
    assert!(restored.writers.is_empty());
    assert!(restored.surfaces.is_empty());
    assert!(restored.control_generation.is_none());
    assert!(restored.confirmation.is_none());
    state.mode = Mode::ConfirmTerminate;
    tab_persistence::save(&mut store, &state).unwrap();
    assert_eq!(
        store.document().tabs.active().view.view,
        TabViewMode::Inventory
    );
}

#[test]
fn unchanged_redraw_does_not_rewrite_the_tab_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap();
    let mut store = TabStore::open(&path, HubId::new()).unwrap();
    let state = tab_persistence::restore(&store);
    let revision = store.document().revision;
    tab_persistence::save(&mut store, &state).unwrap();
    assert_eq!(store.document().revision, revision);
    assert_eq!(
        store.document().tabs.active().view,
        LocalTabView {
            layout: state.tab_layout().clone(),
            view: TabViewMode::Inventory
        }
    );
}

#[test]
fn dirty_gate_saves_first_change_and_skips_unchanged_work() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap();
    let mut store = TabStore::open(&path, HubId::new()).unwrap();
    let mut state = tab_persistence::restore(&store);
    let first_revision = store.document().revision;
    let now = Instant::now();
    let mut saved = tab_persistence::SaveState::default();

    state.mode = Mode::Terminal;
    tab_persistence::save_if_changed_at(&mut store, &state, &mut saved, now).unwrap();
    assert_eq!(store.document().revision, first_revision + 1);

    tab_persistence::save_if_changed_at(&mut store, &state, &mut saved, now).unwrap();
    let revision = store.document().revision;
    tab_persistence::save_if_changed_at(
        &mut store,
        &state,
        &mut saved,
        now + crate::constants::TAB_VERIFY_INTERVAL,
    )
    .unwrap();
    assert_eq!(store.document().revision, revision);
}

#[test]
fn unchanged_gate_defers_replacement_detection_until_verification_deadline() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap();
    let mut store = TabStore::open(&path, HubId::new()).unwrap();
    let state = tab_persistence::restore(&store);
    let mut saved = tab_persistence::SaveState::default();
    let now = Instant::now();
    tab_persistence::save_if_changed_at(&mut store, &state, &mut saved, now).unwrap();

    let file = path.join(yolo_tui_tab::TAB_FILE);
    let replacement = std::fs::read(&file).unwrap();
    std::fs::remove_file(&file).unwrap();
    std::fs::write(&file, &replacement).unwrap();
    tab_persistence::save_if_changed_at(&mut store, &state, &mut saved, now).unwrap();
    assert!(
        tab_persistence::save_if_changed_at(
            &mut store,
            &state,
            &mut saved,
            now + crate::constants::TAB_VERIFY_INTERVAL,
        )
        .is_err()
    );
    assert_eq!(std::fs::read(&file).unwrap(), replacement);
}

#[test]
fn local_edit_after_external_replacement_fails_without_overwriting_it() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap();
    let mut store = TabStore::open(&path, HubId::new()).unwrap();
    let mut state = tab_persistence::restore(&store);
    let mut saved = tab_persistence::SaveState::default();
    let now = Instant::now();
    tab_persistence::save_if_changed_at(&mut store, &state, &mut saved, now).unwrap();

    let file = path.join(yolo_tui_tab::TAB_FILE);
    let replacement = std::fs::read(&file).unwrap();
    std::fs::remove_file(&file).unwrap();
    std::fs::write(&file, &replacement).unwrap();
    state.mode = Mode::Terminal;
    assert!(tab_persistence::save_if_changed_at(&mut store, &state, &mut saved, now).is_err());
    assert_eq!(std::fs::read(&file).unwrap(), replacement);
}

#[test]
fn unchanged_redraw_detects_replacement_and_never_overwrites_it() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap();
    let mut store = TabStore::open(&path, HubId::new()).unwrap();
    let state = tab_persistence::restore(&store);
    let file = path.join(yolo_tui_tab::TAB_FILE);
    let previous = std::fs::read(&file).unwrap();
    std::fs::remove_file(&file).unwrap();
    std::fs::write(&file, &previous).unwrap();
    assert!(tab_persistence::save(&mut store, &state).is_err());
    assert!(tab_persistence::save(&mut store, &state).is_err());
    assert_eq!(std::fs::read(&file).unwrap(), previous);
}

#[test]
fn unchanged_redraw_does_not_recreate_a_deleted_tab_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap();
    let mut store = TabStore::open(&path, HubId::new()).unwrap();
    let state = tab_persistence::restore(&store);
    let file = path.join(yolo_tui_tab::TAB_FILE);
    std::fs::remove_file(&file).unwrap();
    assert!(tab_persistence::save(&mut store, &state).is_err());
    assert!(!file.exists());
}

#[test]
fn terminal_without_a_bound_pane_releases_its_writer_instead_of_renewing_it() {
    let session = fixtures::session();
    let address = session.fields().address;
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.replace_terminals(vec![session.clone()]);
    state.record_grant(fixtures::grant(&session), std::time::Instant::now());
    let (jobs, mut received) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    crate::polling::renew_leases(&mut state, &jobs);
    assert!(state.writers.is_empty());
    assert!(
        matches!(received.try_recv().unwrap(), crate::network::Job::Call { command: yolo_hub_rpc_protocol::Command::Release { grant }, .. } if grant.fields().target.address == address)
    );
    crate::polling::renew_leases(&mut state, &jobs);
    assert!(received.try_recv().is_err());
}
