use crate::host::tests::*;
fn wait_archive_fixture_path(path: &Path) {
    let deadline = std::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT;
    while !path.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "{ARCHIVE_WORKER_FIXTURE_TIMEOUT}"
        );
        std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
    }
}

fn wait_archive_fixture_json(path: &Path) -> ArchiveWorkerFixtureEvidence {
    let deadline = std::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT;
    loop {
        if let Ok(evidence) = read_json(path) {
            return evidence;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{ARCHIVE_WORKER_FIXTURE_TIMEOUT}"
        );
        std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
    }
}

fn wait_archive_fixture_identity_exit(identity: process::ProcessIdentity) {
    let deadline = std::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT;
    while identity.matches() {
        assert!(
            std::time::Instant::now() < deadline,
            "{ARCHIVE_WORKER_FIXTURE_TIMEOUT}"
        );
        std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
    }
}

fn wait_archive_fixture_socket_removal(socket: &Path) {
    let deadline = std::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT;
    while socket.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "{ARCHIVE_WORKER_FIXTURE_TIMEOUT}"
        );
        std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
    }
}

#[test]
fn archive_worker_process_fixture() {
    match std::env::var(ARCHIVE_WORKER_FIXTURE_ROLE_ENV).as_deref() {
        Ok(ARCHIVE_WORKER_FIXTURE_ROLE) => run_archive_worker_fixture(),
        Ok(ARCHIVE_WORKER_REPLACEMENT_ROLE) => run_archive_replacement_fixture(),
        _ => {}
    }
}

fn run_archive_worker_fixture() {
    let root = required_archive_fixture_path(ARCHIVE_WORKER_FIXTURE_ROOT_ENV);
    let socket = required_archive_fixture_path(ARCHIVE_WORKER_FIXTURE_SOCKET_ENV);
    let mut child = spawn_resume_test_child();
    let evidence = ArchiveWorkerFixtureEvidence {
        worker_pid: std::process::id(),
        worker_start_time: process::process_start_time(std::process::id()).unwrap(),
        child_pid: child.id(),
        child_start_time: process::process_start_time(child.id()).unwrap(),
    };
    atomic_write_json(&root.join(ARCHIVE_WORKER_EVIDENCE_FILE), &evidence).unwrap();
    let record = wait_archive_fixture_record(&root.join(ARCHIVE_WORKER_RECORD_FILE));
    let mode = std::env::var(ARCHIVE_WORKER_FIXTURE_MODE_ENV).unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run_archive_worker_server(
            &root, &socket, &record, &mode, &mut child,
        ));
}

fn run_archive_replacement_fixture() {
    let root = required_archive_fixture_path(ARCHIVE_WORKER_FIXTURE_ROOT_ENV);
    let socket = required_archive_fixture_path(ARCHIVE_WORKER_FIXTURE_SOCKET_ENV);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let listener = bind_archive_fixture_listener(&socket);
            std::fs::write(
                root.join(ARCHIVE_REPLACEMENT_READY_FILE),
                ARCHIVE_WORKER_FIXTURE_MARKER,
            )
            .unwrap();
            wait_archive_fixture_path(&root.join(ARCHIVE_REPLACEMENT_RELEASE_FILE));
            drop(listener);
            let _ = std::fs::remove_file(socket);
        });
}

fn wait_archive_fixture_record(path: &Path) -> RuntimeRecord {
    let deadline = std::time::Instant::now() + HOST_BLOCKING_TEST_TIMEOUT;
    loop {
        if let Ok(record) = read_json(path) {
            return record;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{ARCHIVE_WORKER_FIXTURE_TIMEOUT}"
        );
        std::thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
    }
}

fn required_archive_fixture_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}

include!("../test_support/integration/archive_worker_process_fixture_server.rs");
