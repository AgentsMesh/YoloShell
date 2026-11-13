use super::pin::*;
use crate::{constants::PIN_MISMATCH, ensure_running_with_peer};
use yolo_paths::directory::PrivateDirectory;

#[tokio::test]
async fn identity_removed_during_startup_lock_wait_is_not_recreated() {
    let fixture = Fixture::new();
    let peer = fixture.initialize();
    let lock = fixture.lock();
    let startup = ensure_running_with_peer(fixture.launch.clone(), Some(peer), deadline());
    tokio::pin!(startup);
    assert_waiting(startup.as_mut()).await;
    fixture.displace_identity();
    drop(lock);
    let error = startup.await.unwrap_err();
    assert!(error.to_string().contains(PIN_MISMATCH));
    assert_eq!(fixture.inspect(), None);
    assert!(!fixture.launch.host_home.exists());
}

#[tokio::test]
async fn replacement_identity_during_startup_lock_wait_is_not_adopted_or_overwritten() {
    let fixture = Fixture::new();
    let peer = fixture.initialize();
    let lock = fixture.lock();
    let startup = ensure_running_with_peer(fixture.launch.clone(), Some(peer), deadline());
    tokio::pin!(startup);
    assert_waiting(startup.as_mut()).await;
    fixture.displace_identity();
    let replacement = fixture.initialize();
    let bytes = std::fs::read(fixture.identity_path()).unwrap();
    assert_ne!(peer, replacement);
    drop(lock);
    let error = startup.await.unwrap_err();
    assert!(error.to_string().contains(PIN_MISMATCH));
    assert_eq!(fixture.inspect(), Some(replacement));
    assert_eq!(std::fs::read(fixture.identity_path()).unwrap(), bytes);
    assert!(!fixture.launch.host_home.exists());
}

#[tokio::test]
async fn replacing_the_state_directory_during_lock_wait_does_not_create_a_new_identity() {
    let fixture = Fixture::new();
    let peer = fixture.initialize();
    let lock = fixture.lock();
    let startup = ensure_running_with_peer(fixture.launch.clone(), Some(peer), deadline());
    tokio::pin!(startup);
    assert_waiting(startup.as_mut()).await;
    std::fs::rename(
        &fixture.launch.state,
        fixture.root.path().join(SAVED_DIRECTORY),
    )
    .unwrap();
    PrivateDirectory::ensure(&fixture.launch.state).unwrap();
    drop(lock);
    // Retiring the deliberately displaced fixture lock leaves its lifecycle lock.
    let before = std::fs::read_dir(&fixture.launch.state)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(startup.await.is_err());
    assert_eq!(fixture.inspect(), None);
    let after = std::fs::read_dir(&fixture.launch.state)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(after, before);
    assert!(!fixture.launch.host_home.exists());
}
