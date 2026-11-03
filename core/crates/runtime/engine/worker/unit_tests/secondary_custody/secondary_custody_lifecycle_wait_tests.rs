use super::*;

const RELEASE_DELAY: std::time::Duration = std::time::Duration::from_millis(20);

#[test]
fn custody_publication_waits_for_short_lived_lifecycle_readers() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = uuid::Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let namespace = WorkerNamespace::capture(&paths, terminal_id).unwrap();
    let lifecycle = paths
        .try_acquire_session_lifecycle_lock(terminal_id)
        .unwrap()
        .unwrap();
    std::thread::scope(|scope| {
        let releaser = scope.spawn(move || {
            std::thread::sleep(RELEASE_DELAY);
            drop(lifecycle);
        });
        let guard = durable_guard(&namespace, &paths, terminal_id).unwrap();
        guard.require_no_pending_intent().unwrap();
        assert!(
            paths
                .try_acquire_session_lifecycle_lock(terminal_id)
                .unwrap()
                .is_none()
        );
        releaser.join().unwrap();
    });
}
