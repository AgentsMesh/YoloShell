use super::query_support::Fixture;
use crate::{
    Mode,
    constants::{DEFERRED_INPUT_CAPACITY, QUEUE_CAPACITY},
    network::Job,
    terminal_actions, terminal_input,
};
use yolo_hub_rpc_protocol::{Command, Reply};

const SINGLE_JOB: usize = 1;
const PASTE: &[u8] = b"printf '%s\\n' 'complete-paste-with-final-quote'\r";
const OUTPUT: &str = "still receiving terminal output";

fn fixture() -> Fixture {
    let mut fixture = Fixture::with_capacity(SINGLE_JOB);
    fixture.state.mode = Mode::Terminal;
    fixture
}

fn send(fixture: &mut Fixture) -> bool {
    terminal_actions::deliver(&mut fixture.state, &fixture.jobs, Ok(PASTE.to_vec()))
}

fn take(fixture: &mut Fixture) -> yolo_hub_client_channels::ControlRequest {
    let Job::UserInput(request) = fixture.incoming.try_recv().unwrap() else {
        panic!()
    };
    request
}

#[test]
fn saturated_job_queue_preserves_a_whole_paste_once_and_keeps_observation_live() {
    let mut fixture = fixture();
    fixture
        .jobs
        .try_send(Job::Refresh {
            request_id: uuid::Uuid::new_v4(),
        })
        .ok()
        .unwrap();
    assert!(send(&mut fixture));
    assert!(fixture.state.inputs.waiting());
    assert!(!fixture.state.inputs.can_read());
    fixture.emit(OUTPUT.as_bytes());
    assert!(
        fixture.state.surfaces[&fixture.target.address]
            .screen()
            .contents()
            .contains(OUTPUT)
    );
    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        Job::Refresh { .. }
    ));
    terminal_input::flush(&mut fixture.state, fixture.jobs.try_reserve().unwrap());
    assert!(!fixture.state.inputs.waiting());
    let request = take(&mut fixture);
    let Command::Input { request: input } = &request.command else {
        panic!()
    };
    assert_eq!(input.fields().data, PASTE);
    assert_eq!(input.fields().grant, fixture.grant);
    assert_eq!(request.generation, fixture.state.control_generation);
    fixture.finish(request, Ok(Reply::Input { duplicate: false }), true);
    assert!(fixture.state.inputs.pending.is_empty());
    fixture.no_jobs();
}

#[test]
fn unresolved_inputs_bound_memory_and_release_capacity_only_on_exact_completion() {
    let mut fixture = Fixture::with_capacity(QUEUE_CAPACITY);
    fixture.state.mode = Mode::Terminal;
    let mut requests = Vec::new();
    for _ in 0..QUEUE_CAPACITY {
        assert!(send(&mut fixture));
    }
    assert_eq!(fixture.state.inputs.dispatched_len(), QUEUE_CAPACITY);
    for _ in 0..DEFERRED_INPUT_CAPACITY {
        assert!(send(&mut fixture));
    }
    assert_eq!(fixture.state.inputs.deferred.len(), DEFERRED_INPUT_CAPACITY);
    assert_eq!(
        fixture.state.inputs.pending.len(),
        QUEUE_CAPACITY + DEFERRED_INPUT_CAPACITY
    );
    assert!(!fixture.state.inputs.can_read());
    assert!(!send(&mut fixture));
    assert!(
        fixture
            .state
            .inputs
            .overflowed
            .contains(&fixture.target.address)
    );
    for _ in 0..QUEUE_CAPACITY {
        requests.push(take(&mut fixture));
    }
    terminal_input::clear_deferred(&mut fixture.state);
    for request in requests {
        fixture.finish(request, Ok(Reply::Input { duplicate: false }), true);
    }
    assert!(fixture.state.inputs.pending.is_empty());
    assert!(fixture.state.inputs.deferred.is_empty());
    assert!(
        fixture
            .state
            .inputs
            .overflowed
            .contains(&fixture.target.address)
    );
    crate::terminal_input_authority::request_resume(&mut fixture.state, fixture.target.address);
    assert!(
        !fixture
            .state
            .inputs
            .overflowed
            .contains(&fixture.target.address)
    );
    assert!(fixture.state.inputs.can_read());
    assert!(send(&mut fixture));
    assert_eq!(fixture.state.inputs.pending.len(), 1);
}

#[test]
fn a_closed_first_submission_cannot_leave_input_pending_for_replay() {
    let mut fixture = fixture();
    fixture.incoming.close();
    assert!(!send(&mut fixture));
    assert!(fixture.state.inputs.pending.is_empty());
    assert!(!fixture.state.inputs.waiting());
}

#[test]
fn authority_loss_cancels_the_one_unsent_slot_without_retargeting_it() {
    let mut fixture = fixture();
    fixture
        .jobs
        .try_send(Job::Refresh {
            request_id: uuid::Uuid::new_v4(),
        })
        .ok()
        .unwrap();
    assert!(send(&mut fixture));
    fixture.state.disconnected();
    assert!(!fixture.state.inputs.waiting());
    assert!(fixture.state.inputs.pending.is_empty());
    assert!(matches!(
        fixture.incoming.try_recv().unwrap(),
        Job::Refresh { .. }
    ));
    terminal_input::flush(&mut fixture.state, fixture.jobs.try_reserve().unwrap());
    fixture.no_jobs();
}

#[test]
fn closing_the_view_retires_queued_input_before_shutdown_waits_and_never_replays_unknown_input() {
    let mut fixture = fixture();
    assert!(send(&mut fixture));
    let inflight = take(&mut fixture);
    assert!(send(&mut fixture));
    assert!(send(&mut fixture));
    assert!(fixture.state.inputs.waiting());
    fixture.state.inputs.close();
    assert!(!fixture.state.inputs.can_read());
    assert!(!fixture.state.inputs.waiting());
    assert!(fixture.state.inputs.pending.is_empty());
    let queued = take(&mut fixture);
    for request in [&inflight, &queued] {
        assert!(!request.validity.as_ref().unwrap().is_current());
    }
    fixture.finish(
        inflight,
        Err(yolo_hub_client_channels::ControlError::Client(
            yolo_hub_client::ClientError::Remote(
                yolo_hub_terminal_protocol::PublicError::OutcomeUnknown,
            ),
        )),
        false,
    );
    terminal_input::flush(&mut fixture.state, fixture.jobs.try_reserve().unwrap());
    assert!(!send(&mut fixture));
    fixture.no_jobs();
    assert_eq!(
        fixture.state.terminals[&fixture.target.address]
            .fields()
            .runtime
            .unwrap()
            .target,
        fixture.target
    );
}
