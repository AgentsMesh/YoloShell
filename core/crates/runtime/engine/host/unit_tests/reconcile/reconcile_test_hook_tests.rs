use super::*;

const RELATIVE_TEST_ROOT: &str = "relative-test-root";

#[test]
fn barrier_claim_is_one_shot_and_rearm_clears_completion() {
    let root = tempfile::tempdir().unwrap();
    let barrier = ReconcileTestBarrier::new(root.path(), Uuid::new_v4()).unwrap();

    barrier.arm().unwrap();
    assert!(!barrier.reached().unwrap());
    assert!(!barrier.released().unwrap());
    assert!(barrier.claim().unwrap());
    assert!(barrier.reached().unwrap());
    assert!(!barrier.claim().unwrap());
    barrier.release().unwrap();
    assert!(barrier.released().unwrap());

    barrier.arm().unwrap();
    assert!(!barrier.reached().unwrap());
    assert!(!barrier.released().unwrap());
    assert!(ReconcileTestBarrier::new(RELATIVE_TEST_ROOT, Uuid::new_v4()).is_err());
}
