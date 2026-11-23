use crate::host::tests::*;
use super::*;

use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{Notify, mpsc, oneshot};

const TEST_WATCHDOG_INTERVAL: Duration = Duration::from_millis(10);
const TEST_WATCHDOG_BACKOFF: Duration = Duration::from_millis(5);
const TEST_WATCHDOG_TIMEOUT: Duration = Duration::from_secs(2);
const FAILED_RESUME_OBSERVATION_INTERVALS: u32 = 3;

fn test_watchdog_schedule() -> WatchdogSchedule {
    WatchdogSchedule {
        interval: TEST_WATCHDOG_INTERVAL,
        failure_backoff: TEST_WATCHDOG_BACKOFF,
    }
}

#[tokio::test]
async fn watchdog_continues_after_factory_and_future_panics() {
    let factory_attempts = Arc::new(AtomicUsize::new(0));
    let (completion_sender, completion_receiver) = oneshot::channel();
    let completed = Arc::new(Mutex::new(Some(completion_sender)));
    let attempts = Arc::clone(&factory_attempts);
    let signal = Arc::clone(&completed);
    let (shutdown, receiver) = oneshot::channel();
    let task = tokio::spawn(run_watchdog(
        test_watchdog_schedule(),
        receiver,
        move || {
            let attempt = attempts.fetch_add(1, Ordering::SeqCst);
            assert_ne!(attempt, 0, "injected watchdog factory panic");
            let signal = Arc::clone(&signal);
            async move {
                assert_ne!(attempt, 1, "injected watchdog future panic");
                if let Some(sender) = signal.lock().unwrap().take() {
                    let _ = sender.send(());
                }
            }
        },
    ));
    tokio::time::timeout(TEST_WATCHDOG_TIMEOUT, completion_receiver)
        .await
        .unwrap()
        .unwrap();
    shutdown.send(()).unwrap();
    task.await.unwrap();
    assert!(factory_attempts.load(Ordering::SeqCst) >= 3);
}

#[tokio::test]
async fn watchdog_never_overlaps_sweeps() {
    let active = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(Notify::new());
    let (started_sender, mut started_receiver) = mpsc::unbounded_channel();
    let (shutdown, receiver) = oneshot::channel();
    let active_for_sweep = Arc::clone(&active);
    let gate_for_sweep = Arc::clone(&gate);
    let task = tokio::spawn(run_watchdog(
        test_watchdog_schedule(),
        receiver,
        move || {
            let active = Arc::clone(&active_for_sweep);
            let gate = Arc::clone(&gate_for_sweep);
            let started = started_sender.clone();
            async move {
                assert_eq!(active.fetch_add(1, Ordering::SeqCst), 0);
                started.send(()).unwrap();
                gate.notified().await;
                active.fetch_sub(1, Ordering::SeqCst);
            }
        },
    ));
    started_receiver.recv().await.unwrap();
    assert!(
        tokio::time::timeout(TEST_WATCHDOG_INTERVAL, started_receiver.recv())
            .await
            .is_err()
    );
    gate.notify_one();
    tokio::time::timeout(TEST_WATCHDOG_TIMEOUT, started_receiver.recv())
        .await
        .unwrap()
        .unwrap();
    gate.notify_one();
    shutdown.send(()).unwrap();
    task.await.unwrap();
}

#[tokio::test]
async fn contained_client_future_reports_panic_without_propagating() {
    assert!(
        contain_panic(async { panic!("injected client panic") })
            .await
            .is_err()
    );
    assert_eq!(contain_panic(async { 7_u8 }).await.unwrap(), 7);
}

#[tokio::test]
async fn watchdog_parks_a_failed_durable_resume_without_replaying_it() {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join("missing-runtime"));
    let mut host = Host::open(config).unwrap();
    host.watchdog_schedule = test_watchdog_schedule();
    let manifest = fixture_manifest(directory.path());
    let mut record = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join("dead-worker.sock"),
    );
    record.worker_pid = u32::MAX;
    record.worker_start_time = 1;
    record.child_pid = u32::MAX;
    record.child_start_time = 1;
    persist_host_session(&host, &manifest, &record);
    #[cfg(target_os = "linux")]
    crate::host::linux_retirement_evidence::publish(&host, &record);
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = ResumeTransaction::new(
        record.runtime_epoch,
        ResumeIntent::Automatic,
        ResumeTarget::Shell {
            binding: ResumeBindingDisposition::Preserve,
        },
        &launch,
    )
    .unwrap();
    host.publish_resume_transaction(&transaction).unwrap();
    let (published, publication) = oneshot::channel();
    *lock_recover(&host.catalog_test_probes.recovery_publication) = Some(
        RecoveryPublicationProbe::new(manifest.terminal_id, RecoveryState::Uncertain, published),
    );
    let host = Arc::new(host);
    let _watchdog = host.spawn_watchdog();
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, publication)
        .await
        .unwrap()
        .unwrap();
    let launch_path = host.paths.worker_launch(manifest.terminal_id);
    let transaction_path = host.paths.resume_transaction(manifest.terminal_id);
    let manifest_path = host.paths.manifest(manifest.terminal_id);
    let failed_manifest = std::fs::read(&manifest_path).unwrap();
    let failed_launch = std::fs::read(&launch_path).unwrap();
    let failed_transaction = std::fs::read(&transaction_path).unwrap();
    tokio::time::sleep(TEST_WATCHDOG_INTERVAL.saturating_mul(FAILED_RESUME_OBSERVATION_INTERVALS))
        .await;
    assert_eq!(std::fs::read(manifest_path).unwrap(), failed_manifest);
    assert_eq!(std::fs::read(launch_path).unwrap(), failed_launch);
    assert_eq!(std::fs::read(transaction_path).unwrap(), failed_transaction);
}

async fn stalled_status_server(listener: UnixListener, record: RuntimeRecord) {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = UnixStream::from(stream);
    let mut stream = accept_worker_auth(stream, &record).await;
    let _: RequestEnvelope = stream.request().await.unwrap();
    std::future::pending::<()>().await;
}

#[tokio::test]
async fn stalled_live_worker_is_unreachable_but_never_restarted() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.worker_status_timeout = TEST_WATCHDOG_INTERVAL;
    let manifest = fixture_manifest(directory.path());
    let socket = directory.path().join("stalled-worker.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let record = host_runtime_record(&host, manifest.terminal_id, socket);
    persist_host_session(&host, &manifest, &record);
    let server = tokio::spawn(stalled_status_server(listener, record.clone()));
    let result = tokio::time::timeout(TEST_WATCHDOG_TIMEOUT, host.reconcile())
        .await
        .unwrap();
    assert_eq!(result.unreachable, 1);
    assert_eq!(result.needs_recreate, 0);
    assert_eq!(
        read_json::<RuntimeRecord>(&host.paths.runtime_record(manifest.terminal_id))
            .unwrap()
            .runtime_epoch,
        record.runtime_epoch
    );
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
    server.abort();
}

#[path = "watchdog_containment.rs"]
mod watchdog_containment;
#[path = "watchdog_trace_containment.rs"]
mod watchdog_trace_containment;
