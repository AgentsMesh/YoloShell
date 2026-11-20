use crate::worker::tests::*;

#[test]
fn runtime_current_compares_the_held_record_against_the_startup_snapshot() {
    let fixture = Fixture::new();
    let record = fixture.state.record.lock().unwrap();
    assert!(record.matches_worker_runtime(
        record.runtime_epoch,
        record.worker_pid,
        record.worker_start_time,
    ));
    assert!(!record.matches_worker_runtime(
        Uuid::new_v4(),
        record.worker_pid,
        record.worker_start_time,
    ));
    assert!(!record.matches_worker_runtime(
        record.runtime_epoch,
        record.worker_pid.wrapping_add(1),
        record.worker_start_time,
    ));
}
