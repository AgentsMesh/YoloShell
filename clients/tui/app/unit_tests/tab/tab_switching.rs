use super::query_support::*;
use crate::Mode;

#[test]
fn switching_keeps_background_authority_queries_and_exact_bindings() {
    let mut fixture = Fixture::new();
    fixture.state.mode = Mode::Terminal;
    let original = fixture.state.tab_layout().clone();
    fixture.emit(QUERY);
    let before = fixture.queued();
    create(&mut fixture.state, NAME).unwrap();
    assert_eq!(fixture.state.mode, Mode::Inventory);
    assert!(fixture.state.focused_terminal().is_none());
    assert!(
        fixture
            .state
            .bound_terminals()
            .contains(&fixture.target.address)
    );
    crate::polling::renew_leases(&mut fixture.state, &fixture.jobs);
    fixture.no_jobs();
    assert!(
        fixture
            .state
            .grant(fixture.target.address, std::time::Instant::now())
            .is_some()
    );
    assert!(before.validity.unwrap().is_current());
    crate::lease_events::granted(&mut fixture.state, &fixture.jobs, fixture.grant);
    fixture.no_jobs();
    assert!(fixture.state.writers.contains_key(&fixture.target.address));
    fixture.emit(QUERY);
    assert_eq!(fixture.queued().destination, fixture.target.address.hub_id);
    cycle(&mut fixture.state, false);
    assert_eq!(fixture.state.mode, Mode::Terminal);
    assert_eq!(*fixture.state.tab_layout(), original);
    cycle(&mut fixture.state, true);
    assert!(fixture.state.focused_terminal().is_none());
}

#[test]
fn removing_an_inactive_tab_releases_only_its_last_bound_writer() {
    let mut fixture = Fixture::new();
    let first = fixture.state.tabs.active().id;
    create(&mut fixture.state, NAME).unwrap();
    assert!(
        fixture
            .state
            .bound_terminals()
            .contains(&fixture.target.address)
    );
    fixture.state.tabs.remove(first).unwrap();
    fixture.state.sync_query_authority();
    assert!(
        !fixture
            .state
            .tabs
            .entries()
            .iter()
            .any(|entry| entry.id == first)
    );
    assert!(
        !fixture
            .state
            .bound_terminals()
            .contains(&fixture.target.address)
    );
    crate::polling::renew_leases(&mut fixture.state, &fixture.jobs);
    assert!(
        matches!(fixture.incoming.try_recv().unwrap(), crate::network::Job::Call {
        command: yolo_hub_rpc_protocol::Command::Release { grant }, ..
    } if grant == fixture.grant)
    );
    fixture.no_jobs();
    assert!(
        fixture
            .state
            .terminals
            .contains_key(&fixture.target.address)
    );
}

#[test]
fn catalog_restores_all_views_but_never_writer_or_pending_authority() {
    let mut fixture = Fixture::new();
    let original = fixture.state.tab_layout().clone();
    create(&mut fixture.state, NAME).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().canonicalize().unwrap();
    let mut store = yolo_tui_tab::TabStore::open(&path, fixture.target.address.hub_id).unwrap();
    crate::tab_persistence::save(&mut store, &fixture.state).unwrap();
    let mut restored = crate::tab_persistence::restore(&store);
    assert_eq!(restored.tabs.entries().len(), TAB_COUNT);
    assert_eq!(restored.tabs.active().name, NAME);
    assert!(restored.writers.is_empty());
    assert!(restored.queries.scopes.is_empty());
    assert!(restored.control_generation.is_none());
    cycle(&mut restored, false);
    assert_eq!(*restored.tab_layout(), original);
}

#[test]
fn invalid_catalog_insert_does_not_retarget_or_drop_bindings() {
    let mut fixture = Fixture::new();
    let original = fixture.state.tab_layout().clone();
    create(&mut fixture.state, NAME).unwrap();
    let before = fixture.state.tab_snapshot();
    assert!(create(&mut fixture.state, NAME).is_err());
    assert_eq!(fixture.state.tab_snapshot(), before);
    cycle(&mut fixture.state, false);
    assert_eq!(*fixture.state.tab_layout(), original);
    fixture.state.bind_focused(None).unwrap();
    assert!(
        !fixture
            .state
            .bound_terminals()
            .contains(&fixture.target.address)
    );
}

#[test]
fn switching_keeps_filters_but_never_selects_an_invisible_session_for_commands() {
    let mut fixture = Fixture::new();
    create(&mut fixture.state, NAME).unwrap();
    fixture.state.filter = MISSING_FILTER.into();
    cycle(&mut fixture.state, false);
    assert_eq!(
        fixture.state.focused_terminal(),
        Some(fixture.target.address)
    );
    assert_eq!(fixture.state.filter, MISSING_FILTER);
    assert_eq!(fixture.state.selected, None);
    fixture.no_jobs();
}

fn create(state: &mut crate::State, name: &str) -> anyhow::Result<()> {
    let mut catalog = state.tab_snapshot();
    catalog.create(name)?;
    state.tabs = catalog;
    state.activate_tab_collection();
    Ok(())
}
fn cycle(state: &mut crate::State, forward: bool) {
    state.tabs = state.tab_snapshot();
    state.tabs.cycle(forward);
    state.activate_tab_collection();
}
const NAME: &str = "Background work";
const TAB_COUNT: usize = 2;
const QUERY: &[u8] = b"\x1b[5n";
const MISSING_FILTER: &str = "no matching resource";
