fn start_fixture_owner(fixture: &Fixture) -> Option<GatedHostRelease> {
    if let Some(mut guard) = fixture.startup.lock().unwrap().take() {
        let release = GatedHostRelease::respond(fixture);
        pty_owner_handle::spawn_guarded_owner(&mut guard, Arc::clone(&fixture.state)).unwrap();
        return Some(release);
    }
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
    pty_owner_handle::spawn_guarded_owner(&mut guard, Arc::clone(&fixture.state)).unwrap();
    None
}
