use super::{creation_support::*, fixtures};
use crate::{Mode, events, network::Update};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, ControlEvent};
use yolo_hub_terminal_protocol::PublicError;

const CURRENT_STATUS: &str = "Current tab operation completed";

#[test]
fn explicit_rebinding_retires_creation_intent_even_when_the_same_session_returns() {
    for visit_other in [false, true] {
        let mut fixture = Fixture::new();
        let admitted = fixture.start();
        if visit_other {
            let other = fixtures::session();
            let address = other.fields().address;
            fixture.state.terminals.insert(address, other);
            open(&mut fixture, address);
        }
        let original = fixture.expected;
        open(&mut fixture, original);
        let created = fixture.created();
        fixture.finish(admitted.clone(), created);
        assert_eq!(fixture.original_binding(), Some(original));
        assert_eq!(fixture.state.selected, Some(original));
        assert_eq!(fixture.state.mode, Mode::Terminal);
        fixture.assert_finalize(&admitted, created, true);
    }
}

#[test]
fn newer_creation_intent_wins_its_pane_even_when_the_older_request_finishes_first() {
    let mut fixture = Fixture::new();
    let old = fixture.start();
    let current = fixture.start();
    let old_creation = fixture.created();
    fixture.finish(old.clone(), old_creation);
    assert_eq!(fixture.original_binding(), Some(fixture.expected));
    fixture.assert_finalize(&old, old_creation, true);
    let current_creation = fixture.created();
    fixture.finish(current.clone(), current_creation);
    assert_eq!(fixture.original_binding(), Some(current_creation.address));
    fixture.assert_finalize(&current, current_creation, true);
}

#[test]
fn stale_creation_failure_or_not_sent_preserves_the_current_presentation_status() {
    let changes: &[fn(&mut Fixture)] = &[
        |fixture| {
            fixture.switch_tab();
        },
        |fixture| fixture.state.mode = Mode::Search,
        |fixture| fixture.state.filter = CHANGED_FILTER.into(),
        |fixture| fixture.state.inventory.view = yolo_tui_inventory::View::Hubs,
        Fixture::select_other,
        |fixture| {
            let address = fixture.expected;
            open(fixture, address);
            fixture.state.mode = Mode::Inventory;
        },
    ];
    for change in changes {
        for unsent in [false, true] {
            let mut fixture = Fixture::new();
            let request = fixture.start();
            change(&mut fixture);
            fixture.state.status = CURRENT_STATUS.into();
            let update = if unsent {
                Update::NotSent { request }
            } else {
                Update::Control(ControlEvent::Finished {
                    request,
                    result: Err(ControlError::Client(ClientError::Remote(
                        PublicError::Conflict,
                    ))),
                    connected: true,
                })
            };
            events::apply(&mut fixture.state, &fixture.jobs, update);
            assert!(fixture.state.creations.pending.is_empty());
            assert_eq!(fixture.state.status, CURRENT_STATUS);
            assert!(fixture.state.creations.finalizing.is_empty());
        }
    }
}

fn open(fixture: &mut Fixture, address: yolo_hub_identity::TerminalAddress) {
    // Exercise transaction retirement for an explicit destination rebind. UI
    // navigation now preserves other tabs and has its own orchestration tests.
    crate::initial_control::user_event(&mut fixture.state);
    fixture
        .state
        .tab_layout_mut()
        .bind(fixture.pane, Some(address))
        .unwrap();
    fixture
        .state
        .creations
        .retire_binding(fixture.tab, fixture.pane);
    fixture.state.selected = Some(address);
    fixture.state.mode = Mode::Terminal;
    assert!(fixture.incoming.try_recv().is_err());
}
