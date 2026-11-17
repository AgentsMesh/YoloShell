use super::{
    query_support::Fixture, terminal_input_automatic_loss::PREFIX, terminal_input_ordering::queued,
};
use crate::{control, terminal_input_authority};
use ratatui::{Terminal, backend::TestBackend};
use yolo_hub_client_channels::ControlError;
use yolo_hub_terminal_protocol::TerminalSummary;

const WIDTH: u16 = 80;
const STATUS_ROWS: u16 = 1;
const LONG_TITLE: &str =
    "/a/long/project/directory/with/中文/and/many/segments/that/must/not/hide/the/action";
const PAUSED: &str = "Input paused";
const RESUMING: &str = "Resuming input";
const RESUME_ACTION: &str = "Ctrl-b a";
const TAKEOVER_ACTION: &str = "Ctrl-b t";

fn render(fixture: &Fixture) -> String {
    let mut screen = Terminal::new(TestBackend::new(WIDTH, STATUS_ROWS)).unwrap();
    screen
        .draw(|frame| {
            let area = frame.area();
            crate::status_bar::draw(frame, &fixture.state, area, None)
        })
        .unwrap();
    screen
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn interrupted_input_is_paused_and_its_action_survives_a_long_unicode_title() {
    let mut fixture = Fixture::new();
    let request = queued(&mut fixture, PREFIX);
    fixture.finish(request, Err(ControlError::Invalidated), true);
    let address = fixture.target.address;
    let mut fields = fixture.state.terminals[&address].fields().clone();
    fields.presentation.title = Some(LONG_TITLE.into());
    fixture
        .state
        .terminals
        .insert(address, TerminalSummary::new(fields).unwrap());
    assert!(terminal_input_authority::blocked(&fixture.state, address));
    assert_eq!(control::detail(&fixture.state, address), PAUSED);
    let text = render(&fixture);
    assert!(text.contains(PAUSED), "{text}");
    assert!(text.contains(RESUME_ACTION), "{text}");
    assert!(!text.contains("Restoring input"));
    assert!(!text.contains(TAKEOVER_ACTION));
    fixture.no_jobs();
}

#[test]
fn explicit_resume_displays_a_pending_request_not_a_conflicting_owner() {
    let mut fixture = Fixture::new();
    let request = queued(&mut fixture, PREFIX);
    fixture.finish(request, Err(ControlError::Invalidated), true);
    let address = fixture.target.address;
    crate::polling::resume_input(&mut fixture.state, &fixture.jobs, false);
    assert!(!terminal_input_authority::blocked(&fixture.state, address));
    assert!(fixture.state.inputs.interrupted.contains_key(&address));
    assert_eq!(control::detail(&fixture.state, address), RESUMING);
    let text = render(&fixture);
    assert!(text.contains(RESUMING));
    assert!(!text.contains(TAKEOVER_ACTION));
    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        crate::network::Job::Acquire { .. }
    ));
    fixture.no_jobs();
}

#[test]
fn denied_resume_keeps_unknown_input_fenced_and_shows_the_actual_blocker() {
    let mut fixture = Fixture::new();
    let request = queued(&mut fixture, PREFIX);
    fixture.finish(request, Err(ControlError::Invalidated), true);
    let address = fixture.target.address;
    crate::polling::resume_input(&mut fixture.state, &fixture.jobs, false);
    let request = super::control_recovery_support::acquisition(&mut fixture);
    fixture.finish(
        request,
        Err(ControlError::Client(yolo_hub_client::ClientError::Remote(
            yolo_hub_terminal_protocol::PublicError::Unauthorized,
        ))),
        true,
    );
    assert!(fixture.state.inputs.interrupted.contains_key(&address));
    assert!(fixture.state.inputs.resume_requested.contains(&address));
    assert!(fixture.state.writers.is_empty());
    assert_eq!(
        control::detail(&fixture.state, address),
        "Input access denied"
    );
    let text = render(&fixture);
    assert!(text.contains("Check Hub access"), "{text}");
    assert!(!text.contains(RESUMING));
    assert!(!text.contains("Waiting for fresh"));
    control::reconcile(&mut fixture.state, &fixture.jobs);
    fixture.no_jobs();
}

#[test]
fn ended_terminal_shows_lifecycle_actions_instead_of_resuming_a_dead_input_stream() {
    let mut fixture = Fixture::new();
    let request = queued(&mut fixture, PREFIX);
    fixture.finish(request, Err(ControlError::Invalidated), true);
    let address = fixture.target.address;
    fixture
        .state
        .mark_ended(address, fixture.target.runtime_epoch);
    assert!(fixture.state.inputs.interrupted.contains_key(&address));
    assert_eq!(terminal_input_authority::status(&fixture.state), None);
    assert_eq!(control::marker(&fixture.state, address), None);
    let text = render(&fixture);
    assert!(text.contains("Ended"), "{text}");
    assert!(text.contains("Ctrl-b c"), "{text}");
    assert!(!text.contains(RESUME_ACTION));
}
