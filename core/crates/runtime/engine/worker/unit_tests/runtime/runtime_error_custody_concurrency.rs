const RUNTIME_ERROR_CONCURRENT_SCENARIOS: usize = 4;
#[cfg(any(not(test), feature = "yolo-test-support"))]
include!("../../test_support/integration/runtime_error/runtime_error_process_watchdog.rs");
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
include!("../../integration_tests/runtime/runtime_error_process_watchdog_adversarial.rs");

struct ConcurrentRuntimeErrorScenario {
    paths: Paths,
    terminal_id: Uuid,
    worker: RuntimeErrorWorker,
    directory: GuardianRuntimeDirectory,
}

impl ConcurrentRuntimeErrorScenario {
    fn spawn() -> Self {
        let directory = GuardianRuntimeDirectory::new();
        let paths = Paths::new(directory.path());
        let terminal_id = Uuid::new_v4();
        let launch = launch_spec(&paths, terminal_id);
        write_launch(&paths, terminal_id, &launch);
        let worker = RuntimeErrorWorker::spawn(paths.clone(), terminal_id);
        Self {
            directory,
            paths,
            terminal_id,
            worker,
        }
    }

    async fn complete(mut self) -> (GuardianRuntimeDirectory, Paths) {
        complete_runtime_error_custody(&self.paths, self.terminal_id, &mut self.worker).await;
        let error = self.worker.finish_error().await;
        assert!(
            error.to_string().contains(BOOTSTRAP_EXEC_FAILURE_CONTEXT),
            "{error:#}"
        );
        let record: RuntimeRecord = read_json(&self.paths.runtime_record(self.terminal_id)).unwrap();
        assert_eq!(record.bootstrap_delivery, BootstrapDelivery::Failed);
        assert!(!self.paths.worker_launch(self.terminal_id).exists());
        assert!(!process::process_identity_matches(
            record.child_pid,
            record.child_start_time,
        ));
        (self.directory, self.paths)
    }
}

#[test]
fn concurrent_runtime_errors_release_each_exact_host_custody() -> Result<()> {
    let processes = (0..RUNTIME_ERROR_CONCURRENT_SCENARIOS)
        .map(|_| ConcurrentRuntimeErrorProcess::spawn())
        .collect::<Result<Vec<_>>>()?;
    for process in processes {
        process.finish()?;
    }
    Ok(())
}

#[test]
fn concurrent_runtime_error_process_helper() {
    if std::env::var(RUNTIME_ERROR_PROCESS_HELPER_ENV).as_deref()
        != Ok(RUNTIME_ERROR_PROCESS_HELPER_ROLE)
    {
        return;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (directory, paths) =
        runtime.block_on(async { ConcurrentRuntimeErrorScenario::spawn().complete().await });
    // A Worker process also shuts down its executor before its durable home is retired.
    // Joining run() alone need not drop independently cancelled ingress listeners yet.
    drop(runtime);
    directory.close(&paths).unwrap();
}
