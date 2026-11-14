const TEST_STARTUP_LIFECYCLE_TIMEOUT: Duration = crate::session_timing::WORKER_READY_TIMEOUT;
const TEST_SHUTDOWN_LIFECYCLE_TIMEOUT: Duration =
    WORKER_AUTH_TIMEOUT.saturating_add(WORKER_TERMINATION_TIMEOUT);
const TEST_WORKER_EARLY_EXIT_DETAIL: &str = "adversarial worker exited before runtime publication";

async fn wait_for_adversarial_worker(
    paths: &Paths,
    terminal_id: Uuid,
    marker: &Path,
    worker: &mut tokio::task::JoinHandle<Result<()>>,
) -> RuntimeRecord {
    tokio::time::timeout(TEST_STARTUP_LIFECYCLE_TIMEOUT, async {
        loop {
            if worker.is_finished() {
                let outcome = worker.await;
                panic!("{TEST_WORKER_EARLY_EXIT_DETAIL}: {outcome:?}");
            }
            if let Ok(record) = read_json::<RuntimeRecord>(&paths.runtime_record(terminal_id))
                && marker.exists()
                && let Ok(response) = rpc(
                    &record,
                    record.controller_generation,
                    WORKER_STATUS_METHOD,
                    &(),
                )
                .await
                && response.ok
                && let Ok(status) = response.decode_payload::<RuntimeView>()
                && status.status == TerminalRuntimeStatus::Running
            {
                return record;
            }
            tokio::time::sleep(TEST_READY_POLL_INTERVAL).await;
        }
    })
    .await
    .unwrap()
}

fn retire_adversarial_launch(paths: &Paths, terminal_id: Uuid) {
    let path = paths.worker_launch(terminal_id);
    crate::paths::private_file::remove_private_file(&path).unwrap();
    assert!(!path.exists());
}
