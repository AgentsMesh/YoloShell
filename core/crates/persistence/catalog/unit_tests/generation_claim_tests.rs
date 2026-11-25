use super::*;
use std::sync::{Arc, Barrier};
use tempfile::tempdir;

const CATALOG_FILE_NAME: &str = "catalog.sqlite";
const CLAIMANT_COUNT: usize = 2;
const FIRST_GENERATION_OFFSET: u64 = 1;
const GENERATION_FLOOR: u64 = 41;
const EXHAUSTION_FLOOR: u64 = u64::MAX - 1;
const EXPECTED_EXHAUSTED_CLAIMS: usize = 1;

#[test]
fn concurrent_generation_claims_are_atomic_and_monotonic() {
    let directory = tempdir().unwrap();
    let path = directory.path().join(CATALOG_FILE_NAME);
    drop(Catalog::open(&path).unwrap());
    let barrier = Arc::new(Barrier::new(CLAIMANT_COUNT));
    let catalogs = (0..CLAIMANT_COUNT)
        .map(|_| Catalog::open(&path).unwrap())
        .collect::<Vec<_>>();
    let mut claimants = Vec::with_capacity(CLAIMANT_COUNT);
    for mut catalog in catalogs {
        let barrier = Arc::clone(&barrier);
        claimants.push(std::thread::spawn(move || {
            barrier.wait();
            catalog
                .claim_controller_generation(GENERATION_FLOOR)
                .unwrap()
        }));
    }
    let mut generations = claimants
        .into_iter()
        .map(|claimant| claimant.join().unwrap())
        .collect::<Vec<_>>();
    generations.sort_unstable();

    assert_eq!(
        generations,
        [
            GENERATION_FLOOR + FIRST_GENERATION_OFFSET,
            GENERATION_FLOOR + CLAIMANT_COUNT as u64,
        ]
    );
    let catalog = Catalog::open(&path).unwrap();
    assert_eq!(
        catalog.controller_generation().unwrap(),
        GENERATION_FLOOR + CLAIMANT_COUNT as u64
    );
}

#[test]
fn concurrent_claim_at_generation_limit_has_one_winner_and_never_wraps() {
    let directory = tempdir().unwrap();
    let path = directory.path().join(CATALOG_FILE_NAME);
    let mut catalog = Catalog::open(&path).unwrap();
    catalog
        .restore_controller_generation_floor(EXHAUSTION_FLOOR)
        .unwrap();
    drop(catalog);
    let barrier = Arc::new(Barrier::new(CLAIMANT_COUNT));
    let mut claimants = Vec::with_capacity(CLAIMANT_COUNT);
    for mut catalog in (0..CLAIMANT_COUNT).map(|_| Catalog::open(&path).unwrap()) {
        let barrier = Arc::clone(&barrier);
        claimants.push(std::thread::spawn(move || {
            barrier.wait();
            catalog.bump_controller_generation()
        }));
    }
    let outcomes = claimants
        .into_iter()
        .map(|claimant| claimant.join().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_err()).count(),
        EXPECTED_EXHAUSTED_CLAIMS
    );
    assert_eq!(
        outcomes
            .into_iter()
            .filter_map(Result::ok)
            .collect::<Vec<_>>(),
        [u64::MAX]
    );
    let mut reopened = Catalog::open(&path).unwrap();
    assert_eq!(reopened.controller_generation().unwrap(), u64::MAX);
    assert!(reopened.bump_controller_generation().is_err());
}
