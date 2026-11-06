use crate::worker::tests::*;
include!("runtime_error_custody_concurrency.rs");

#[test]
fn bootstrap_forwards_an_unregistered_provider_environment() {
    const FUTURE_DATA_HOME_ENV: &str = "FUTURE_AGENT_HOME";
    const FUTURE_ENVIRONMENT_VALUE: &str = "/future-agent/home";
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let mut launch = launch_spec(&paths, Uuid::new_v4());
    launch.environment = [(
        FUTURE_DATA_HOME_ENV.to_owned(),
        FUTURE_ENVIRONMENT_VALUE.to_owned(),
    )]
    .into();
    let spec = bootstrap_child_spec(&launch, Uuid::new_v4()).unwrap();
    let command = bootstrap_command(&launch, &spec, None, None, None).unwrap();
    assert_eq!(
        command.command.get_env(FUTURE_DATA_HOME_ENV),
        Some(std::ffi::OsStr::new(FUTURE_ENVIRONMENT_VALUE))
    );
}

#[tokio::test]
async fn worker_run_rejects_missing_and_invalid_launch_contracts() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    assert!(run(paths.clone(), terminal_id).await.is_err());

    let mut launch = launch_spec(&paths, Uuid::new_v4());
    write_launch(&paths, terminal_id, &launch);
    assert!(run(paths.clone(), terminal_id).await.is_err());

    launch.terminal_id = terminal_id;
    launch.schema_version = WORKER_LAUNCH_SCHEMA_VERSION + 1;
    write_launch(&paths, terminal_id, &launch);
    assert!(run(paths.clone(), terminal_id).await.is_err());

    launch.schema_version = WORKER_LAUNCH_SCHEMA_VERSION;
    launch.socket_path = paths.worker_socket(Uuid::new_v4()).unwrap();
    write_launch(&paths, terminal_id, &launch);
    let error = run(paths.clone(), terminal_id).await.unwrap_err();
    assert!(
        error
            .to_string()
            .contains(WORKER_SOCKET_NAMESPACE_MISMATCH_CONTEXT)
    );

    launch.socket_path =
        std::path::PathBuf::from(format!("/tmp/{}", "x".repeat(UNIX_SOCKET_PATH_MAX_BYTES)));
    write_launch(&paths, terminal_id, &launch);
    assert!(run(paths, terminal_id).await.is_err());
}

#[tokio::test]
async fn worker_run_preserves_launch_when_the_retained_journal_cannot_open() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    let launch = launch_spec(&paths, terminal_id);
    write_launch(&paths, terminal_id, &launch);
    let journal_path = paths.journal(terminal_id);
    std::fs::create_dir(&journal_path).unwrap();

    assert!(run(paths.clone(), terminal_id).await.is_err());
    assert!(paths.worker_launch(terminal_id).exists());
    assert!(journal_path.is_dir());
    assert!(!launch.socket_path.exists());
}

#[test]
fn worker_run_surfaces_spawn_and_stale_socket_errors() {
    let directory = GuardianRuntimeDirectory::new();
    let paths = Paths::new(directory.path());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(assert_worker_spawn_failure(&paths));
    drop(runtime);
    directory.close(&paths).unwrap();
}

async fn assert_worker_spawn_failure(paths: &Paths) {
    let terminal_id = Uuid::new_v4();
    let launch = launch_spec(paths, terminal_id);
    write_launch(paths, terminal_id, &launch);
    let mut worker = RuntimeErrorWorker::spawn(paths.clone(), terminal_id);
    complete_runtime_error_custody(paths, terminal_id, &mut worker).await;
    let error = worker.finish_error().await;
    assert!(
        error.to_string().contains(BOOTSTRAP_EXEC_FAILURE_CONTEXT),
        "{error:#}"
    );
    let record: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    assert_eq!(record.bootstrap_delivery, BootstrapDelivery::Failed);
    assert!(!paths.worker_launch(terminal_id).exists());
    assert!(!process::process_identity_matches(
        record.child_pid,
        record.child_start_time,
    ));
}

#[tokio::test]
async fn worker_run_preserves_stale_socket_on_bind_error() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    let mut launch = launch_spec(&paths, terminal_id);
    launch.command = "/bin/true".into();
    launch.environment.clear();
    write_launch(&paths, terminal_id, &launch);
    let _namespace_guard = paths.retain_socket_namespace().unwrap();
    std::fs::create_dir_all(&launch.socket_path).unwrap();
    let socket_parent = launch.socket_path.parent().unwrap();
    let parent_identity = crate::paths::identity::ObjectIdentity::from_metadata(
        &std::fs::symlink_metadata(socket_parent).unwrap(),
    );
    let socket_identity = crate::paths::identity::ObjectIdentity::from_metadata(
        &std::fs::symlink_metadata(&launch.socket_path).unwrap(),
    );
    assert!(run(paths, terminal_id).await.is_err());
    assert_eq!(
        crate::paths::identity::ObjectIdentity::from_metadata(
            &std::fs::symlink_metadata(socket_parent).unwrap(),
        ),
        parent_identity
    );
    assert_eq!(
        crate::paths::identity::ObjectIdentity::from_metadata(
            &std::fs::symlink_metadata(&launch.socket_path).unwrap(),
        ),
        socket_identity
    );
    std::fs::remove_dir(&launch.socket_path).unwrap();
}

#[test]
fn runtime_publication_is_excluded_by_lifecycle_mutation_and_archived_state() {
    let fixture = Fixture::new();
    let record = lock_recover(&fixture.state.record).clone();
    let runtime_path = fixture.state.paths.runtime_record(record.terminal_id);
    fixture.state.namespace.remove_file(&runtime_path).unwrap();
    let lifecycle_lock = fixture
        .state
        .paths
        .try_acquire_session_lifecycle_lock(record.terminal_id)
        .unwrap()
        .unwrap();

    let busy = publish_runtime_record(&fixture.state.namespace, &fixture.state.paths, &record)
        .unwrap_err();
    assert!(busy.to_string().contains(RUNTIME_PUBLICATION_LOCK_CONTEXT));
    assert!(!runtime_path.exists());
    drop(lifecycle_lock);

    let manifest_path = fixture.state.paths.manifest(record.terminal_id);
    let mut manifest: crate::model::TerminalManifest = read_json(&manifest_path).unwrap();
    manifest.archived = true;
    atomic_write_json(&manifest_path, &manifest).unwrap();
    let archived = publish_runtime_record(&fixture.state.namespace, &fixture.state.paths, &record)
        .unwrap_err();
    assert!(archived.to_string().contains(RUNTIME_PUBLICATION_REJECTED));
    assert!(!runtime_path.exists());
}
