use super::{constants::*, fixture::Fixture};
use std::sync::{Arc, Barrier, Mutex};
use yolo_hub_ssh_profile_protocol_test_support::{change, configuration};
use yolo_hub_ssh_profiles::{RevisionConflict, SshProfileConfiguration};

#[test]
fn concurrent_clients_have_one_cas_winner_and_second_store_cannot_take_ownership() {
    let fixture = Fixture::new();
    let store = fixture.open(None).unwrap();
    assert!(fixture.open(None).is_err());
    let state = store.current().unwrap();
    let requests = [
        change(&state, configuration()),
        change(&state, SshProfileConfiguration::default()),
    ];
    let shared = Arc::new(Mutex::new(store));
    let barrier = Arc::new(Barrier::new(TWO_CLIENTS));
    let results = std::thread::scope(|scope| {
        let workers = requests
            .into_iter()
            .map(|request| {
                let store = shared.clone();
                let barrier = barrier.clone();
                scope.spawn(move || {
                    barrier.wait();
                    store.lock().unwrap().apply(request)
                })
            })
            .collect::<Vec<_>>();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(
        results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .all(|error| error.is::<RevisionConflict>())
    );
    let winner = results.into_iter().find_map(Result::ok).unwrap();
    assert_eq!(shared.lock().unwrap().current().unwrap(), winner);
}
