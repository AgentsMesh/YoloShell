use super::*;
use std::time::Duration;

const RELEASE_DELAY: Duration = Duration::from_millis(40);

#[test]
fn successor_checkpoint_waits_for_predecessor_lifecycle_release() {
    let fixture = CheckpointFixture::new();
    let lifecycle = fixture
        .paths
        .try_acquire_session_lifecycle_lock(fixture.launch.terminal_id)
        .unwrap()
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(RELEASE_DELAY);
        drop(lifecycle);
    });

    let mutation = mutation::CheckpointMutation::acquire(
        &fixture.paths,
        fixture.namespace.lease(),
        fixture.launch.terminal_id,
    )
    .unwrap();

    drop(mutation);
    release.join().unwrap();
}

#[test]
fn successor_checkpoint_rejects_an_aba_replacement_while_waiting() {
    let fixture = CheckpointFixture::new();
    fixture.begin();
    let (evidence, identity) = path_io::read_path(&fixture.paths, fixture.launch.terminal_id)
        .unwrap()
        .unwrap();
    let lifecycle = fixture
        .paths
        .try_acquire_session_lifecycle_lock(fixture.launch.terminal_id)
        .unwrap()
        .unwrap();
    let path = fixture.path();
    let replacement = evidence.clone();
    let release = std::thread::spawn(move || {
        std::thread::sleep(RELEASE_DELAY);
        crate::paths::atomic_write_json(&path, &replacement).unwrap();
        drop(lifecycle);
    });

    let result = path_io::replace_path(&fixture.paths, identity, &evidence);

    release.join().unwrap();
    let error = result.unwrap_err();
    assert!(format!("{error:#}").contains(constants::STALE_ERROR));
}
