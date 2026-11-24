use crate::worker::tests::*;
const PUBLICATION_CONTENTION_OBSERVATION: Duration = Duration::from_millis(50);
const PUBLICATION_CONTENTION_COMPLETION: Duration = Duration::from_secs(2);
const PUBLICATION_COLUMN_INCREMENT: u16 = 1;

#[test]
fn runtime_publication_waits_for_a_live_participant_instead_of_failing_startup() {
    let fixture = Fixture::new();
    let mut expected = lock_recover(&fixture.state.record).clone();
    expected.cols += PUBLICATION_COLUMN_INCREMENT;
    let held = fixture
        .state
        .paths
        .try_acquire_session_lifecycle_lock(fixture.terminal_id())
        .unwrap()
        .unwrap();
    let state = Arc::clone(&fixture.state);
    let next = expected.clone();
    let (started, startup) = std::sync::mpsc::channel();
    let (finished, completion) = std::sync::mpsc::channel();
    let publisher = std::thread::spawn(move || {
        started.send(()).unwrap();
        finished
            .send(publish_runtime_record(
                &state.namespace,
                &state.paths,
                &next,
            ))
            .unwrap();
    });
    startup
        .recv_timeout(PUBLICATION_CONTENTION_COMPLETION)
        .unwrap();
    assert!(matches!(
        completion.recv_timeout(PUBLICATION_CONTENTION_OBSERVATION),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ));
    drop(held);
    completion
        .recv_timeout(PUBLICATION_CONTENTION_COMPLETION)
        .unwrap()
        .unwrap();
    publisher.join().unwrap();
    let actual: RuntimeRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.runtime_record(fixture.terminal_id()))
        .unwrap();
    assert_eq!(actual, expected);
}
