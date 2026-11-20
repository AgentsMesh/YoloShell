use super::{
    query_support::Fixture,
    terminal_input_automatic_loss::{PREFIX, SUFFIX},
    terminal_input_ordering::queued,
};
use crate::{
    Mode, constants::CONNECTED_LABEL, control, polling, terminal_actions,
    terminal_input_authority::*,
};
use ratatui::{Terminal, backend::TestBackend};
use yolo_hub_client_channels::ControlError;

const COLUMNS: u16 = 120;
const ROWS: u16 = 2;

#[test]
fn interrupted_input_remains_visible_and_never_reacquires_from_refresh_or_navigation() {
    let mut fixture = Fixture::new();
    let request = queued(&mut fixture, PREFIX);
    fixture.finish(request, Err(ControlError::Invalidated), true);
    assert!(blocked(&fixture.state, fixture.target.address));
    fixture.state.status = CONNECTED_LABEL.into();
    for _ in [(), (), ()] {
        polling::renew_leases(&mut fixture.state, &fixture.jobs);
        polling::acquire(&mut fixture.state, &fixture.jobs, false);
        control::reconcile(&mut fixture.state, &fixture.jobs);
    }
    fixture.no_jobs();
    assert!(!terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(SUFFIX.to_vec())
    ));
    fixture.state.mode = Mode::Inventory;
    fixture.state.sync_query_authority();
    fixture.state.mode = Mode::Terminal;
    fixture.state.sync_query_authority();
    assert_eq!(status(&fixture.state), Some(INTERRUPTED_HINT));
    let mut screen = Terminal::new(TestBackend::new(COLUMNS, ROWS)).unwrap();
    screen
        .draw(|frame| {
            let area = frame.area();
            crate::status_bar::draw(frame, &fixture.state, area, None)
        })
        .unwrap();
    let rendered: String = screen
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(rendered.contains(INTERRUPTED_HINT));
}

#[test]
fn unsolicited_new_grant_does_not_acknowledge_an_interrupted_stream() {
    let mut fixture = Fixture::new();
    let request = queued(&mut fixture, PREFIX);
    fixture.finish(request, Err(ControlError::Invalidated), true);
    fixture.replace_writer();
    assert!(fixture.state.writers.is_empty());
    assert!(blocked(&fixture.state, fixture.target.address));
    assert_eq!(status(&fixture.state), Some(INTERRUPTED_HINT));
    fixture.no_jobs();
}
