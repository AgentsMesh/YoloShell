use super::startup_support::*;
use crate::{Mode, StartupAction};
use yolo_hub_terminal_protocol::RuntimeStatus;

#[test]
fn retired_session_uri_does_not_attach_or_create_a_terminal() {
    use super::fixtures::{ADDRESS_SCHEME, REMOVED_TERMINAL_RESOURCE};
    let mut f = Fixture::new(StartupAction::Inventory);
    let terminal = session(f.hub, RuntimeStatus::Running);
    let address = terminal.fields().address;
    let selector = format!(
        "{ADDRESS_SCHEME}://{}/{REMOVED_TERMINAL_RESOURCE}/{}",
        address.hub_id, address.terminal_id
    );
    f.state.replace_terminals(vec![terminal]);
    crate::startup::install(
        &mut f.state,
        f.hub,
        StartupAction::Attach {
            selector: Some(selector),
        },
    );
    f.ready();
    f.apply();
    f.no_jobs();
    assert!(f.state.focused_terminal().is_none());
    assert!(f.state.creations.pending.is_empty());
}

#[test]
fn attach_selectors_match_exact_unique_name_id_or_origin_qualified_address() {
    for kind in 0..4 {
        let mut f = Fixture::new(StartupAction::Inventory);
        let shell = session(f.hub, RuntimeStatus::Running);
        let address = shell.fields().address;
        let selector = match kind {
            0 => NAME.into(),
            1 => address.terminal_id.to_string(),
            2 => format!("{}/{}", address.hub_id, address.terminal_id),
            _ => address.to_string(),
        };
        f.state.replace_terminals(vec![shell.clone()]);
        crate::startup::install(
            &mut f.state,
            f.hub,
            StartupAction::Attach {
                selector: Some(selector),
            },
        );
        f.ready();
        f.apply();
        f.acquired(&shell);
        assert_eq!(f.state.focused_terminal(), Some(address));
    }
}

#[test]
fn ambiguous_or_missing_attach_never_creates_or_grants_input() {
    for selector in [None, Some(NAME.into()), Some(MISSING.into())] {
        let mut f = Fixture::new(StartupAction::Attach { selector });
        f.state.replace_terminals(vec![
            session(f.hub, RuntimeStatus::Running),
            session(f.hub, RuntimeStatus::Running),
        ]);
        f.ready();
        f.apply();
        f.no_jobs();
        assert_eq!(f.state.mode, Mode::Inventory);
        assert!(f.state.focused_terminal().is_none());
        assert!(f.state.creations.pending.is_empty());
    }
}

#[test]
fn attach_without_selector_routes_an_expired_saved_terminal_to_history() {
    let mut f = Fixture::new(StartupAction::Attach { selector: None });
    let shell = session(f.hub, RuntimeStatus::Exited);
    let address = shell.fields().address;
    f.state.bind_focused(Some(address)).unwrap();
    crate::startup::install(
        &mut f.state,
        f.hub,
        StartupAction::Attach { selector: None },
    );
    f.state.replace_terminals(vec![shell]);
    assert_eq!(f.state.focused_terminal(), None);
    assert!(f.state.sidebar_visible);
    f.ready();
    f.apply();
    f.no_jobs();
    assert_eq!(f.state.focused_terminal(), None);
    assert!(f.state.terminal_picker.is_some());
    assert!(f.state.inventory.history);
    assert_eq!(f.state.selected, Some(address));
    assert_eq!(f.state.status, crate::startup_constants::ENDED_SESSION);
}

#[test]
fn a_coherent_inventory_owns_keys_during_an_attach_retry() {
    let mut f = Fixture::new(StartupAction::Attach { selector: None });
    f.state.replace_terminals(vec![
        session(f.hub, RuntimeStatus::Running),
        session(f.hub, RuntimeStatus::Running),
    ]);
    f.ready();
    f.apply();
    f.no_jobs();
    assert_eq!(f.state.mode, Mode::Inventory);
    assert!(f.state.inventory.current);

    crate::actions::handle(
        &mut f.state,
        &f.jobs,
        f.hub,
        crossterm::event::Event::Key(crossterm::event::KeyCode::Down.into()),
    );

    assert!(f.state.inputs.pregrant.is_empty());
}
