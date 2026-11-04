use crate::{constants::*, fixture::*, model::*};
use std::{
    fs,
    sync::{Arc, Barrier},
};
use yolo_hub_client_access_store::ClientAccessStore;
use yolo_paths::directory::PrivateDirectory;

#[test]
fn replacing_the_coordination_lock_revokes_the_old_owner_without_touching_configuration() {
    let fixture = Fixture::new();
    let mut store = fixture.open(Some(configuration())).unwrap();
    let original = store.current().unwrap();
    let bytes = fixture.bytes();
    let lock_bytes = fs::read(fixture.lock()).unwrap();
    let retired = fixture.sibling(RETIRED_LOCK);
    fs::rename(fixture.lock(), &retired).unwrap();
    write_new(&fixture.lock(), &lock_bytes);
    assert!(store.replace(original.revision, next(&original)).is_err());
    assert_poisoned(&mut store, &original);
    assert_eq!(fixture.bytes(), bytes);
    fs::rename(fixture.lock(), fixture.sibling(REPLACEMENT_LOCK)).unwrap();
    fs::rename(&retired, fixture.lock()).unwrap();
    assert_poisoned(&mut store, &original);
    drop(store);
    assert_eq!(fixture.open(None).unwrap().current().unwrap(), original);
}

#[test]
fn simultaneous_openers_cannot_both_acquire_or_commit_access_configuration() {
    let fixture = Fixture::new();
    let barrier = Arc::new(Barrier::new(OPENERS));
    let tasks: Vec<_> = (0..OPENERS)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let path = fixture.path.clone();
            std::thread::spawn(move || {
                let directory = PrivateDirectory::open(&path).unwrap();
                barrier.wait();
                ClientAccessStore::open(directory, hub(), Some(configuration()))
            })
        })
        .collect();
    let results: Vec<_> = tasks.into_iter().map(|task| task.join().unwrap()).collect();
    assert_eq!(
        results.iter().filter(|result| result.is_ok()).count(),
        ONE_OWNER
    );
    let mut owner = results.into_iter().find_map(Result::ok).unwrap();
    let original = owner.current().unwrap();
    let expected = next(&original);
    assert_eq!(
        owner.replace(original.revision, expected.clone()).unwrap(),
        expected
    );
    drop(owner);
    assert_eq!(fixture.open(None).unwrap().current().unwrap(), expected);
}
