use crate::worker::tests::*;
#[tokio::test]
async fn attach_barrier_filters_output_already_in_initial_replay() {
    let fixture = Fixture::new();
    let mut receiver = fixture.state.events.subscribe();
    let included = fixture.state.commit_output(b"included").unwrap();
    fixture
        .state
        .events
        .send(WorkerEvent::Output(included.clone()))
        .unwrap();
    let attached = fixture.state.attach_result_internal(0).unwrap();
    assert_eq!(attached.chunks.len(), 1);
    assert_eq!(attached.chunks[0].sequence, included.sequence);
    assert_eq!(attached.chunks[0].data, included.data);
    let barrier = attached.next_sequence.saturating_sub(1);

    let live = fixture.state.commit_output(b"live").unwrap();
    fixture
        .state
        .events
        .send(WorkerEvent::Output(live.clone()))
        .unwrap();
    let WorkerEvent::Output(forwarded) = recv_after_barrier(&mut receiver, barrier).await.unwrap()
    else {
        panic!("expected a terminal output event");
    };
    assert_eq!(forwarded.sequence, live.sequence);
    assert_eq!(forwarded.data, live.data);
}
