use super::{constants::*, fixture::Fixture};
use std::sync::{Arc, Barrier};
use uuid::Uuid;

#[test]
fn same_operation_concurrent_publishers_converge_without_rewriting_the_release() {
    let fixture = Fixture::new();
    let (stage, descriptor) = fixture.stage(FIRST_FILL);
    let operation = Uuid::new_v4();
    let barrier = Arc::new(Barrier::new(RACE_PARTICIPANTS));
    let results = std::thread::scope(|scope| {
        let workers = (0..RACE_PARTICIPANTS)
            .map(|_| {
                let store = fixture.store();
                let barrier = Arc::clone(&barrier);
                let descriptor = &descriptor;
                scope.spawn(move || {
                    barrier.wait();
                    store.publish(stage, descriptor, None, operation)
                })
            })
            .collect::<Vec<_>>();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert!(results.iter().any(Result::is_ok));
    let selected = fixture.store().inspect().unwrap().unwrap();
    for installed in results.into_iter().filter_map(Result::ok) {
        assert_eq!(installed.selection(), &selected);
    }
    let replay = fixture
        .store()
        .publish(stage, &descriptor, None, operation)
        .unwrap();
    assert_eq!(replay.selection(), &selected);
}

#[test]
fn different_builds_cannot_both_commit_against_the_same_empty_selector() {
    let fixture = Fixture::new();
    let candidates = [fixture.stage(FIRST_FILL), fixture.stage(SECOND_FILL)];
    let barrier = Arc::new(Barrier::new(RACE_PARTICIPANTS));
    let results = std::thread::scope(|scope| {
        let workers = candidates
            .iter()
            .map(|(stage, descriptor)| {
                let store = fixture.store();
                let barrier = Arc::clone(&barrier);
                scope.spawn(move || {
                    barrier.wait();
                    store.publish(*stage, descriptor, None, Uuid::new_v4())
                })
            })
            .collect::<Vec<_>>();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    let committed = results
        .iter()
        .filter_map(|result| result.as_ref().ok())
        .collect::<Vec<_>>();
    assert_eq!(committed.len(), 1);
    let selection = fixture.store().inspect().unwrap().unwrap();
    assert_eq!(committed[0].selection(), &selection);
    let (stage, descriptor) = candidates
        .iter()
        .find(|(_, descriptor)| descriptor.build_id() != selection.build_id())
        .unwrap();
    assert!(
        fixture
            .store()
            .publish(*stage, descriptor, None, Uuid::new_v4())
            .is_err()
    );
    assert_eq!(fixture.store().inspect().unwrap(), Some(selection));
}
