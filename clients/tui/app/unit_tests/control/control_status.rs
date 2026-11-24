use super::{panes, query_support::Fixture};
use crate::{Mode, actions, control, input_intent, pointer};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use std::time::{SystemTime, UNIX_EPOCH};
use yolo_hub_terminal_protocol::{LeaseObservation, LeaseObservationFields};

const HOLDER: &str = "other-client";
const EXPIRY_MILLIS: i64 = 60_000;

fn observation(fixture: &Fixture) -> LeaseObservation {
    LeaseObservation::new(LeaseObservationFields {
        target: fixture.target,
        generation: fixture.grant.fields().generation,
        holder_id: HOLDER.into(),
        controller_instance_id: None,
        expires_at_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
            + EXPIRY_MILLIS,
    })
    .unwrap()
}

const SINGLE_JOB: usize = 1;

#[test]
fn control_detail_distinguishes_observe_only_and_an_observed_holder() {
    let mut fixture = Fixture::new();
    let address = fixture.target.address;
    fixture.state.writers.clear();
    fixture.state.control.observe(fixture.target);
    assert!(control::detail(&fixture.state, address).contains("Ctrl-b t"));
    let lease = observation(&fixture);
    fixture
        .state
        .control
        .record_observation(fixture.target, Some(&lease));
    assert!(control::detail(&fixture.state, address).contains(HOLDER));
    fixture
        .state
        .control
        .disconnected(std::time::Instant::now());
    assert!(control::detail(&fixture.state, address).contains("Ctrl-b t"));
}

#[test]
fn own_acquisition_does_not_offer_takeover_again() {
    let mut fixture = Fixture::new();
    let address = fixture.target.address;
    fixture.state.writers.clear();

    let detail = control::detail(&fixture.state, address);

    assert_eq!(detail, "Restoring input");
    assert!(!detail.contains("take control"));
}

#[test]
fn terminal_payload_backpressure_does_not_classify_navigation_as_a_payload() {
    let mut fixture = Fixture::new();
    fixture.state.mode = Mode::Inventory;
    let arrow = Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert!(!input_intent::is_terminal_payload(&fixture.state, &arrow));
    fixture.state.mode = Mode::Terminal;
    assert!(input_intent::is_terminal_payload(&fixture.state, &arrow));
}

#[test]
fn resize_and_focus_events_preserve_deferred_terminal_bytes() {
    let mut fixture = Fixture::with_capacity(SINGLE_JOB);
    fixture.state.mode = Mode::Terminal;
    fixture
        .jobs
        .try_send(crate::network::Job::Refresh {
            request_id: uuid::Uuid::new_v4(),
        })
        .unwrap();
    assert!(crate::terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"prefix".to_vec())
    ));
    assert!(crate::terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"suffix".to_vec())
    ));
    assert_eq!(fixture.state.inputs.deferred.len(), 1);
    let (jobs, _) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    actions::handle(
        &mut fixture.state,
        &jobs,
        fixture.target.address.hub_id,
        Event::Resize(100, 40),
    );
    actions::handle(
        &mut fixture.state,
        &jobs,
        fixture.target.address.hub_id,
        Event::FocusLost,
    );
    assert_eq!(fixture.state.inputs.deferred.len(), 1);
}

#[test]
fn terminal_mouse_payload_requires_provider_mouse_protocol() {
    let (mut state, first, second) = panes::state();
    let focused = state.tab_layout().focused();
    let target = state
        .presentation
        .panes
        .iter()
        .find(|view| view.pane == focused)
        .and_then(|view| view.terminal)
        .map(|address| {
            if address == first.fields().address {
                &first
            } else {
                &second
            }
        })
        .unwrap();
    panes::writable(&mut state, target, b"");
    let view = state
        .presentation
        .panes
        .iter()
        .find(|view| view.pane == state.tab_layout().focused())
        .copied()
        .unwrap();
    let event = panes::mouse(MouseEventKind::ScrollUp, view.inner.x, view.inner.y);
    assert!(!input_intent::is_terminal_payload(
        &state,
        &Event::Mouse(event)
    ));
    panes::writable(&mut state, target, panes::ENABLE_MOUSE);
    assert!(input_intent::is_terminal_payload(
        &state,
        &Event::Mouse(event)
    ));
}

#[test]
fn captured_mouse_release_remains_terminal_payload_outside_the_pane() {
    let (mut state, first, second) = panes::state();
    let focused = state.tab_layout().focused();
    let target = state
        .presentation
        .panes
        .iter()
        .find(|view| view.pane == focused)
        .and_then(|view| view.terminal)
        .map(|address| {
            if address == first.fields().address {
                &first
            } else {
                &second
            }
        })
        .unwrap();
    panes::writable(&mut state, target, panes::ENABLE_MOUSE);
    let view = state
        .presentation
        .panes
        .iter()
        .find(|view| view.pane == state.tab_layout().focused())
        .copied()
        .unwrap();
    let (jobs, mut received) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        panes::mouse(
            MouseEventKind::Down(MouseButton::Left),
            view.inner.x,
            view.inner.y,
        ),
    );
    assert!(received.try_recv().is_ok());
    let release = panes::mouse(MouseEventKind::Up(MouseButton::Left), 0, 0);
    assert!(input_intent::is_terminal_payload(
        &state,
        &Event::Mouse(release)
    ));
}
