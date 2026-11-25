use super::support::*;
use std::sync::{Arc, Barrier};
use yolo_hub_client_access_registry::{AccessRegistry, RegistryError};

const COMPETING_WRITERS: usize = 2;
const ONE_WINNER: usize = 1;

#[test]
fn concurrent_cas_admits_exactly_one_policy_and_never_revives_prior_admissions() {
    let original = configuration();
    let registry = AccessRegistry::new(original.clone()).unwrap();
    let admission = registry.admit_verified_key(&CLIENT_KEY, NOW_MS).unwrap();
    let barrier = Arc::new(Barrier::new(COMPETING_WRITERS));
    let writers: Vec<_> = [CLIENT_KEY, OTHER_KEY]
        .into_iter()
        .map(|key| {
            let registry = registry.clone();
            let barrier = barrier.clone();
            let mut candidate = next(&original);
            let expected = original.revision;
            candidate.clients[FIRST_CLIENT].identity.public_key = key;
            std::thread::spawn(move || {
                barrier.wait();
                registry.replace(expected, candidate)
            })
        })
        .collect();
    let outcomes: Vec<_> = writers
        .into_iter()
        .map(|writer| writer.join().unwrap())
        .collect();
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_ok()).count(),
        ONE_WINNER
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == Err(RegistryError::RevisionConflict))
            .count(),
        ONE_WINNER
    );
    assert_eq!(
        registry.current().unwrap().revision,
        original.revision + REVISION_STEP
    );
    assert!(!admission.is_current(NOW_MS));
    let configured = registry.current().unwrap().clients.remove(FIRST_CLIENT);
    let current = registry
        .admit_verified_key(&configured.identity.public_key, NOW_MS)
        .unwrap();
    assert_eq!(current.grant(), &configured);
    assert!(current.is_current(NOW_MS));
}
