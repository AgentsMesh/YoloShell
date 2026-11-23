use super::intake_test_support::*;
use crate::codex_hooks::CodexHookIntake;
use std::sync::{Arc, Barrier};

#[test]
fn occurrence_publication_is_exclusive() {
    let fixture = IntakeFixture::new();
    let directory_path = fixture.artifacts.intake_directory();
    let path = directory_path.join(EXCLUSIVE_FILE_NAME);
    let barrier = Arc::new(Barrier::new(CONCURRENT_WRITERS));
    let successes = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..CONCURRENT_WRITERS)
            .map(|index| {
                let barrier = Arc::clone(&barrier);
                let directory_path = directory_path.clone();
                let path = path.clone();
                scope.spawn(move || {
                    let directory =
                        crate::paths::directory::PrivateDirectory::ensure(&directory_path).unwrap();
                    barrier.wait();
                    super::super::intake_storage::create(&directory, &path, &index).is_ok()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .filter(|succeeded| *succeeded)
            .count()
    });
    assert_eq!(successes, 1);
    let directory = crate::paths::directory::PrivateDirectory::open(&directory_path).unwrap();
    let (bytes, _) = super::super::intake_storage::read_bytes(&directory, &path).unwrap();
    let installed: usize = serde_json::from_slice(&bytes).unwrap();
    assert!(installed < CONCURRENT_WRITERS);
}

#[test]
fn concurrent_helpers_publish_unique_occurrences() {
    let fixture = IntakeFixture::new();
    let directory = fixture.artifacts.intake_directory();
    let owner = fixture.identity.clone();
    let barrier = Arc::new(Barrier::new(CONCURRENT_WRITERS));
    std::thread::scope(|scope| {
        for _ in 0..CONCURRENT_WRITERS {
            let barrier = Arc::clone(&barrier);
            let directory = directory.clone();
            let owner = owner.clone();
            scope.spawn(move || {
                let intake = CodexHookIntake::for_identity(directory, &owner).unwrap();
                barrier.wait();
                intake.persist(envelope(&owner)).unwrap();
            });
        }
    });
    let scan = fixture.intake().scan().unwrap();
    assert_eq!(scan.occurrences.len(), CONCURRENT_WRITERS);
    let ids: std::collections::BTreeSet<_> = scan
        .occurrences
        .iter()
        .map(|occurrence| occurrence.envelope().event_id)
        .collect();
    assert_eq!(ids.len(), CONCURRENT_WRITERS);
}
