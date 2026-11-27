const ORPHAN_STARTUP_DIAGNOSTIC_KIB: u64 = 8;
const ORPHAN_STARTUP_DIAGNOSTIC_BYTES_PER_KIB: u64 = 1024;
const ORPHAN_STARTUP_DIAGNOSTIC_LIMIT: u64 =
    ORPHAN_STARTUP_DIAGNOSTIC_KIB * ORPHAN_STARTUP_DIAGNOSTIC_BYTES_PER_KIB;
const ORPHAN_STARTUP_DIAGNOSTIC_READ_ERROR: &str = "child stderr could not be read";
const ORPHAN_CUSTODY_RUNNING: &str = "running";
const ORPHAN_CUSTODY_COMPLETED: &str = "completed";
const ORPHAN_CUSTODY_FAILED: &str = "failed";
const ORPHAN_CUSTODY_PANICKED: &str = "panicked";

fn fail_orphan_exit(
    reason: &str,
    observed_status: Option<std::process::ExitStatus>,
    child: &mut std::process::Child,
    paths: &Paths,
    terminal_id: Uuid,
    custody: &mut Option<std::thread::JoinHandle<Result<()>>>,
    child_stderr: &mut std::fs::File,
) -> ! {
    let exit = read_json::<ExitRecord>(&paths.exit_record(terminal_id))
        .map(|value| format!("{:?}/{:?}", value.reason, value.signal))
        .unwrap_or_else(|error| format!("unavailable: {error:#}"));
    let runtime = read_json::<RuntimeRecord>(&paths.runtime_record(terminal_id))
        .map(|value| format!("{:?}", value.bootstrap_delivery))
        .unwrap_or_else(|error| format!("unavailable: {error:#}"));
    let custody = orphan_custody_state(custody);
    let child_stderr = read_orphan_child_stderr(child_stderr);
    let status = observed_status.or_else(|| {
        let _ = child.kill();
        child.wait().ok()
    });
    panic!(
        "{reason}: child_status={status:?}; exit={exit}; \
         runtime={runtime}; custody={custody}; child_stderr={child_stderr:?}"
    );
}

fn orphan_custody_state(custody: &mut Option<std::thread::JoinHandle<Result<()>>>) -> String {
    if !custody.as_ref().is_some_and(|thread| thread.is_finished()) {
        return String::from(ORPHAN_CUSTODY_RUNNING);
    }
    match custody.take().unwrap().join() {
        Ok(Ok(())) => String::from(ORPHAN_CUSTODY_COMPLETED),
        Ok(Err(error)) => format!("{ORPHAN_CUSTODY_FAILED}: {error:#}"),
        Err(_) => String::from(ORPHAN_CUSTODY_PANICKED),
    }
}

fn fail_orphan_startup(
    reason: &str,
    observed_status: Option<std::process::ExitStatus>,
    child: &mut std::process::Child,
    paths: &Paths,
    terminal_id: Uuid,
    custody: &mut Option<std::thread::JoinHandle<Result<()>>>,
    child_stderr: &mut std::fs::File,
) -> ! {
    let status = observed_status.or_else(|| {
        let _ = child.kill();
        child.wait().ok()
    });
    let checkpoint = read_json::<startup_checkpoint::StartupCheckpointEvidence>(
        &paths.worker_startup_checkpoint(terminal_id),
    )
    .map(|value| format!("{:?}/{:?}/{:?}", value.stage, value.phase, value.status))
    .unwrap_or_else(|error| format!("unavailable: {error:#}"));
    let runtime = read_json::<RuntimeRecord>(&paths.runtime_record(terminal_id))
        .map(|value| format!("{:?}", value.bootstrap_delivery))
        .unwrap_or_else(|error| format!("unavailable: {error:#}"));
    let custody = orphan_custody_state(custody);
    let child_stderr = read_orphan_child_stderr(child_stderr);
    panic!(
        "{reason}: child_status={status:?}; checkpoint={checkpoint}; runtime={runtime}; custody={custody}; child_stderr={child_stderr:?}"
    );
}

fn read_orphan_child_stderr(stderr: &mut std::fs::File) -> String {
    if std::io::Seek::rewind(stderr).is_err() {
        return String::from(ORPHAN_STARTUP_DIAGNOSTIC_READ_ERROR);
    }
    let mut output = Vec::new();
    let mut bounded = std::io::Read::take(stderr, ORPHAN_STARTUP_DIAGNOSTIC_LIMIT);
    if std::io::Read::read_to_end(&mut bounded, &mut output).is_err() {
        return String::from(ORPHAN_STARTUP_DIAGNOSTIC_READ_ERROR);
    }
    String::from_utf8_lossy(&output).into_owned()
}
