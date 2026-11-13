use super::support::error_text;
use crate::WorkspaceState;
use std::fs;
use std::sync::{Arc, Barrier};
use yolo_release_workspace_state_test_support::{
    DIRTY_TRACKED_BYTES, PARALLEL_WORKSPACE_CASES, TRACKED_FILE, repository,
};

#[test]
fn parallel_dirty_workspace_checks_are_isolated_and_deterministic() {
    let repositories = (0..PARALLEL_WORKSPACE_CASES)
        .map(|_| repository())
        .collect::<Vec<_>>();
    let start = Arc::new(Barrier::new(PARALLEL_WORKSPACE_CASES));
    let workers = repositories
        .into_iter()
        .map(|repository| {
            let start = Arc::clone(&start);
            std::thread::spawn(move || {
                fs::write(repository.path().join(TRACKED_FILE), DIRTY_TRACKED_BYTES).unwrap();
                start.wait();
                error_text(WorkspaceState::validate(repository.path()))
            })
        })
        .collect::<Vec<_>>();
    for worker in workers {
        let error = worker.join().unwrap();
        assert!(error.contains("not clean"), "{error}");
    }
}
