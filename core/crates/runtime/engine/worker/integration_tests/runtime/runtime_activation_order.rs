use crate::worker::tests::*;
const RUNTIME_STAGE_CLEANUP_DETAIL: &str =
    "runtime activation staging did not release its startup process";

#[test]
fn reader_preparation_failure_preserves_the_startup_monitor() {
    let fixture = Fixture::new();
    let root = fixture_root_identity(&fixture);
    let child = fixture.child.lock().unwrap().take().unwrap();
    let mut guard =
        StartupChildGuard::with_reader_backed_output(child, fixture_runtime_output(&fixture));
    guard.track(root).unwrap();
    let (master, writer) = take_runtime_stage_io(&fixture);
    let mut startup = StartupPtyResources::from_guard(master, guard);
    let prepared = prepared_runtime_for_stage(&fixture, writer);

    assert!(prepared.stage(&mut startup).is_err());
    startup.child().refresh().unwrap();

    drop(startup);
    assert_runtime_stage_cleanup(root);
}

#[test]
fn ownership_staging_stops_the_monitor_only_after_reader_preparation() {
    let fixture = Fixture::new();
    let root = fixture_root_identity(&fixture);
    let output = {
        let master = lock_recover(&fixture.state.master);
        crate::worker::pty_output_progress::ExclusivePtyOutput::capture(
            master.as_ref().unwrap().as_ref(),
            Arc::clone(&fixture.state.journal),
        )
        .unwrap()
    };
    let child = fixture.child.lock().unwrap().take().unwrap();
    let mut guard = StartupChildGuard::with_output(child, output);
    guard.track(root).unwrap();
    let (master, writer) = take_runtime_stage_io(&fixture);
    let mut startup = StartupPtyResources::from_guard(master, guard);
    let prepared = prepared_runtime_for_stage(&fixture, writer);

    let staged = prepared.stage(&mut startup).unwrap();
    assert!(startup.child().refresh().is_err());

    drop(staged);
    drop(startup);
    assert_runtime_stage_cleanup(root);
}

fn prepared_runtime_for_stage(
    fixture: &Fixture,
    writer: PtyInputWriter,
) -> PreparedWorkerRuntime {
    PreparedWorkerRuntime {
        state: Arc::clone(&fixture.state),
        reader: Box::new(std::io::empty()),
        writer,
    }
}

fn take_runtime_stage_io(
    fixture: &Fixture,
) -> (Box<dyn MasterPty + Send>, PtyInputWriter) {
    let master = lock_recover(&fixture.state.master).take().unwrap();
    let writer = lock_recover(&fixture.state.writer).take().unwrap();
    (master, writer)
}

fn assert_runtime_stage_cleanup(root: process::ProcessIdentity) {
    let deadline = Instant::now() + WORKER_TERMINATION_TIMEOUT;
    while root.matches() {
        assert!(Instant::now() < deadline, "{RUNTIME_STAGE_CLEANUP_DETAIL}");
        std::thread::sleep(TEST_READY_POLL_INTERVAL);
    }
}
