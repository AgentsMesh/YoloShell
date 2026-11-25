use super::*;

pub(super) fn take_fault_ownership(fixture: &Fixture) -> pty_ownership::PtyOwnership {
    let child = fixture.child.lock().unwrap().take().unwrap();
    let record = lock_recover(&fixture.state.record);
    let root = process::ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    };
    drop(record);
    let mut guard =
        StartupChildGuard::with_reader_backed_output(child, fixture_runtime_output(fixture));
    guard.track(root).unwrap();
    let ownership = guard.take_ownership().unwrap();
    guard.disarm_after_transfer();
    ownership
}
