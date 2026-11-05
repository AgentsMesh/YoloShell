use super::query_support::Fixture;
use crate::{
    Mode,
    constants::{DEFERRED_INPUT_CAPACITY, QUEUE_CAPACITY},
    network::Job,
    terminal_actions, terminal_input,
};
use yolo_hub_rpc_protocol::Reply;

const DEFERRED_WINDOW: usize = DEFERRED_INPUT_CAPACITY - 1;

#[test]
fn one_receipt_releases_one_dispatched_slot_for_fifo_deferred_input() {
    let mut fixture = Fixture::with_capacity(QUEUE_CAPACITY);
    fixture.state.mode = Mode::Terminal;
    for _ in 0..QUEUE_CAPACITY {
        assert!(terminal_actions::deliver(
            &mut fixture.state,
            &fixture.jobs,
            Ok(b"sent".to_vec())
        ));
    }
    for _ in 0..DEFERRED_WINDOW {
        assert!(terminal_actions::deliver(
            &mut fixture.state,
            &fixture.jobs,
            Ok(b"deferred".to_vec())
        ));
    }
    assert_eq!(fixture.state.inputs.dispatched_len(), QUEUE_CAPACITY);
    assert_eq!(fixture.state.inputs.deferred.len(), DEFERRED_WINDOW);
    let requests = (0..QUEUE_CAPACITY)
        .map(|_| match fixture.incoming.try_recv().unwrap() {
            Job::UserInput(request) => request,
            _ => panic!("expected terminal input"),
        })
        .collect::<Vec<_>>();
    fixture.finish(
        requests[0].clone(),
        Ok(Reply::Input { duplicate: false }),
        true,
    );
    assert!(terminal_input::deferred_ready(&fixture.state));
    terminal_input::flush(&mut fixture.state, fixture.jobs.try_reserve().unwrap());
    assert!(matches!(fixture.incoming.try_recv(), Ok(Job::UserInput(_))));
    assert_eq!(fixture.state.inputs.dispatched_len(), QUEUE_CAPACITY);
    assert_eq!(fixture.state.inputs.deferred.len(), DEFERRED_WINDOW - 1);
}
