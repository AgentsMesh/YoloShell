use super::*;
use crate::host::tests::*;

const INSERT_CORRUPT_CATALOG_ROW_SQL: &str = "INSERT INTO sessions(
    session_id, manifest_json, updated_at_ms, archived
) VALUES(?1, ?2, ?3, 0)";
const UNCERTAIN_READINESS_BUDGET: Duration = Duration::from_secs(2);
const UNCERTAIN_READY_TIMEOUT: Duration = creation_launch_test_timeout(UNCERTAIN_READINESS_BUDGET);
const UNCERTAIN_READY_POLL: Duration = Duration::from_millis(5);

#[tokio::test]
async fn poisoned_host_mutexes_and_bad_rows_do_not_hide_healthy_sessions() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    host.catalog.lock().unwrap().insert(&manifest).unwrap();
    atomic_write_json(&host.paths.manifest(manifest.terminal_id), &manifest).unwrap();
    rusqlite::Connection::open(host.paths.catalog())
        .unwrap()
        .execute(
            INSERT_CORRUPT_CATALOG_ROW_SQL,
            rusqlite::params!["invalid-row-key", b"invalid-json", now_ms()],
        )
        .unwrap();
    let catalog_poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _catalog = host.catalog.lock().unwrap();
        panic!("injected catalog poison");
    }));
    let locks_poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _entries = host.session_locks.entries.lock().unwrap();
        panic!("injected session lock poison");
    }));
    assert!(catalog_poisoned.is_err());
    assert!(locks_poisoned.is_err());

    let guard = tokio::time::timeout(
        HOST_BLOCKING_TEST_TIMEOUT,
        host.session_locks.lock(manifest.terminal_id),
    )
    .await
    .unwrap();
    drop(guard);
    let response = host
        .dispatch(&request(
            method::LIST_SESSIONS,
            &ListSessionsRequest {
                include_archived: false,
                require_complete: false,
            },
        ))
        .await;
    let listed: ListSessionsResult = response.decode_payload().unwrap();
    assert_eq!(listed.sessions.len(), 1);
    assert_eq!(
        listed.sessions[0].manifest.terminal_id,
        manifest.terminal_id
    );
    assert!(!listed.complete);
    let complete = host
        .dispatch(&request(
            method::LIST_SESSIONS,
            &ListSessionsRequest {
                include_archived: true,
                require_complete: true,
            },
        ))
        .await;
    assert!(!complete.ok);
    assert_eq!(
        complete.error.unwrap().code,
        crate::protocol::error_code::INTERNAL
    );
}

#[tokio::test]
async fn live_child_identity_respects_automatic_restore_policy() {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join("missing-runtime"));
    let host = Host::open(config).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut record = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join("dead-worker.sock"),
    );
    record.worker_pid = u32::MAX;
    record.worker_start_time = 1;
    persist_host_session(&host, &manifest, &record);

    let result = host.reconcile().await;
    assert_eq!(result.unreachable, 1);
    assert_eq!(result.needs_recreate, 0);
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
    let stored = lock_recover(&host.catalog)
        .get(manifest.terminal_id)
        .unwrap()
        .unwrap();
    assert_eq!(stored.recovery_state, RecoveryState::Uncertain);
}

#[tokio::test]
async fn crossed_spawn_boundary_never_confirms_an_uninitialized_creation() {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(PathBuf::from(crate::test_executables::TRUE));
    let mut host = Host::open(config).unwrap();
    host.worker_ready_timeout = UNCERTAIN_READY_TIMEOUT;
    host.worker_ready_poll_interval = UNCERTAIN_READY_POLL;

    let request = CreateTerminal {
        creation_token: Uuid::new_v4(),
        name: Some("uncertain-start".into()),
        cwd: directory.path().to_path_buf(),
        cols: None,
        rows: None,
        initial_input: None,
        context: None,
    };
    let started = std::time::Instant::now();
    let error = host.create_session(request.clone()).await.unwrap_err();
    let elapsed = started.elapsed();
    assert_eq!(error.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    let listed = lock_recover(&host.catalog).list(false).unwrap();
    assert_eq!(listed.len(), 1);
    let diagnostic = (listed[0].recovery_state != RecoveryState::Uncertain)
        .then(|| crossed_spawn_diagnostic::capture(&host, &request, &listed[0], elapsed));
    assert_eq!(
        listed[0].recovery_state,
        RecoveryState::Uncertain,
        "{diagnostic:?}"
    );
    let terminal_id = listed[0].terminal_id;
    let launch_path = host.paths.worker_launch(terminal_id);
    let uncertain_launch = std::fs::read(&launch_path).unwrap();
    let error = host
        .resume_session(ResumeRequest {
            selection: TerminalLifecycleSelection::unconditional(terminal_id),
            recreate: true,
        })
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(std::fs::read(launch_path).unwrap(), uncertain_launch);
    assert_eq!(host.reconcile().await.needs_recreate, 0);
}

#[tokio::test]
async fn resume_never_replaces_a_socket_that_is_still_serving() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    lock_recover(&host.catalog).insert(&manifest).unwrap();
    atomic_write_json(&host.paths.manifest(manifest.terminal_id), &manifest).unwrap();
    let socket = host.paths.worker_socket(manifest.terminal_id).unwrap();
    let (listener, publication) = publish_test_socket(&host.socket_namespace, &socket).await;

    let error = host
        .resume_session_locked(manifest.terminal_id, ResumeSelection::ExplicitRecreate)
        .await
        .map_err(SessionResumeFailure::into_explicit_error)
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert!(socket.exists());
    assert!(socket_may_be_serving(&socket).await);
    drop(listener);
    drop(publication);
}
#[path = "../../test_support/unit/crossed_spawn_diagnostic.rs"]
mod crossed_spawn_diagnostic;
