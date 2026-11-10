use super::{creation_support::*, fixtures};
use crate::Mode;
use crossterm::event::{Event, KeyCode};

#[test]
fn later_user_selection_filter_scope_view_or_mode_is_never_reset_by_creation_reply() {
    let changes: &[fn(&mut Fixture)] = &[
        Fixture::select_other,
        |f| {
            f.state.filter = CHANGED_FILTER.into();
        },
        |f| {
            f.state.inventory.scope = Some(yolo_hub_identity::HubId::new());
        },
        |f| {
            f.state.inventory.view = yolo_tui_inventory::View::Hubs;
        },
        |f| {
            f.state.mode = Mode::Search;
        },
        |f| {
            f.state.tab_manager = Some(yolo_tui_tab_manager::Manager::new(&f.state.tabs));
        },
    ];
    for change in changes {
        let mut fixture = Fixture::new();
        let request = fixture.start();
        change(&mut fixture);
        let (selected, filter, scope, view, mode) = (
            fixture.state.selected,
            fixture.state.filter.clone(),
            fixture.state.inventory.scope,
            fixture.state.inventory.view,
            fixture.state.mode,
        );
        let created = fixture.created();
        fixture.finish(request.clone(), created);
        assert_eq!(fixture.original_binding(), Some(created.address));
        assert_eq!(
            (
                fixture.state.selected,
                fixture.state.filter.clone(),
                fixture.state.inventory.scope,
                fixture.state.inventory.view,
                fixture.state.mode
            ),
            (selected, filter, scope, view, mode)
        );
        fixture.assert_finalize(&request, created, true);
    }
}

#[test]
fn catalog_selection_changes_preserve_the_pending_creation_presentation() {
    let mut fixture = Fixture::new();
    let request = fixture.start();
    let epoch = fixture.state.ui_epoch;
    fixture.state.replace_terminals(vec![fixtures::session()]);
    assert_ne!(fixture.state.selected, Some(fixture.expected));
    assert_eq!(fixture.state.ui_epoch, epoch);
    let created = fixture.created();
    fixture.finish(request.clone(), created);
    assert_eq!(fixture.original_binding(), Some(created.address));
    assert_eq!(fixture.state.selected, Some(created.address));
    assert!(fixture.state.initial_control.is_some());
    fixture.assert_finalize(&request, created, true);
}

#[test]
fn selection_round_trip_does_not_restore_creation_presentation_authority() {
    let mut fixture = Fixture::new();
    let request = fixture.start();
    fixture.select_other();
    let key = if fixture.state.visible().first() == Some(&fixture.expected) {
        KeyCode::Up
    } else {
        KeyCode::Down
    };
    crate::actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        Event::Key(key.into()),
    );
    assert_eq!(fixture.state.selected, Some(fixture.expected));
    let created = fixture.created();
    fixture.finish(request.clone(), created);
    assert_eq!(fixture.original_binding(), Some(created.address));
    assert_eq!(fixture.state.selected, Some(fixture.expected));
    assert!(fixture.state.initial_control.is_none());
    fixture.assert_finalize(&request, created, true);
}
