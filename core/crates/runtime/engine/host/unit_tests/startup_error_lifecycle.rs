use crate::host::tests::*;
use super::*;

const STARTUP_ARM_INGRESS_ERROR: &str = "injected startup ingress cleanup failure";
const STARTUP_ARM_CLEANUP_NOT_STARTED: &str = "startup ingress cleanup did not start";
const STARTUP_ARM_CLEANUP_RETURNED_EARLY: &str = "startup ingress cleanup returned early";
const STARTUP_ARM_CLEANUP_NOT_FINISHED: &str = "startup ingress cleanup did not finish";
const STARTUP_ARM_HOST_OWNER_SURVIVED: &str = "startup ingress retained Host ownership";
const STARTUP_MAINTENANCE_ERROR: &str = "injected maintenance startup failure";
const STARTUP_MAINTENANCE_CLEANUP_ERROR: &str = "injected maintenance cleanup failure";
const STARTUP_MAINTENANCE_FAILURE_COUNT: usize = 2;
const STARTUP_SCANNER_LABEL: &str = "startup-error-scanner";
const STARTUP_SCANNER_OWNER: &str = "startup-error-owner.lock";
const STARTUP_SCANNER_ENTRY_LOST: &str = "startup scanner entry was not observed";
const STARTUP_SCANNER_RELEASE_LOST: &str = "startup scanner was not released";
const STARTUP_SCANNER_NOT_DROPPED: &str = "startup scanner was not dropped";

struct StartupErrorScanner {
    entered: Option<std::sync::mpsc::SyncSender<()>>,
    release: std::sync::mpsc::Receiver<()>,
    dropped: Option<tokio::sync::oneshot::Sender<()>>,
}

impl crate::maintenance::MaintenanceScanner for StartupErrorScanner {
    fn label(&self) -> &'static str {
        STARTUP_SCANNER_LABEL
    }

    fn scan_chunk(&mut self) -> Result<crate::maintenance::ScanChunk> {
        if let Some(entered) = self.entered.take() {
            entered
                .send(())
                .map_err(|_| anyhow::anyhow!(STARTUP_SCANNER_ENTRY_LOST))?;
        }
        self.release
            .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
            .map_err(|_| anyhow::anyhow!(STARTUP_SCANNER_RELEASE_LOST))?;
        Ok(crate::maintenance::ScanChunk::PassComplete)
    }

    fn reset(&mut self) {}
}

impl Drop for StartupErrorScanner {
    fn drop(&mut self) {
        if let Some(dropped) = self.dropped.take() {
            let _ = dropped.send(());
        }
    }
}

#[test]
fn maintenance_start_and_cleanup_errors_preserve_both_failures() {
    let error = combine_maintenance_start_errors(
        anyhow::anyhow!(STARTUP_MAINTENANCE_ERROR),
        Err(anyhow::anyhow!(STARTUP_MAINTENANCE_CLEANUP_ERROR)),
    )
    .unwrap_err();
    let aggregate = error.downcast_ref::<HostShutdownErrors>().unwrap();
    assert_eq!(
        aggregate.failures.len(),
        STARTUP_MAINTENANCE_FAILURE_COUNT
    );
    let mut failures = aggregate.failures.iter();
    let startup = failures.next().unwrap();
    let cleanup = failures.next().unwrap();
    assert!(failures.next().is_none());
    assert_eq!(startup.stage, HostShutdownStage::Maintenance);
    assert_eq!(cleanup.stage, HostShutdownStage::Maintenance);
    assert!(
        format!("{:#}", startup.error).contains(STARTUP_MAINTENANCE_ERROR)
    );
    assert!(
        format!("{:#}", cleanup.error).contains(STARTUP_MAINTENANCE_CLEANUP_ERROR)
    );
}

#[tokio::test]
async fn ready_failure_awaits_ingress_cleanup_and_preserves_both_errors() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let owner = Arc::clone(&host);
    let (armed, ready) = tokio::sync::oneshot::channel::<IngressArmed>();
    let (shutdown, receiver) = tokio::sync::oneshot::channel();
    let (entered, cleanup_started) = tokio::sync::oneshot::channel();
    let (release, cleanup_release) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        drop(armed);
        let _owner = owner;
        let _ = receiver.await;
        let _ = entered.send(());
        let _ = cleanup_release.await;
        Err(anyhow::anyhow!(STARTUP_ARM_INGRESS_ERROR))
    });
    let mut ingress = HostIngress {
        task,
        shutdown: Some(shutdown),
    };
    let stopping = tokio::spawn(async move {
        let service = ready.await.context(HOST_INGRESS_ARM_CONTEXT).map(|_| ());
        stop_ingress_after_startup_failure(&mut ingress, service, Ok(())).await
    });
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, cleanup_started)
        .await
        .expect(STARTUP_ARM_CLEANUP_NOT_STARTED)
        .expect(STARTUP_ARM_CLEANUP_NOT_STARTED);
    assert!(!stopping.is_finished(), "{STARTUP_ARM_CLEANUP_RETURNED_EARLY}");
    release.send(()).unwrap();
    let error = tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, stopping)
        .await
        .expect(STARTUP_ARM_CLEANUP_NOT_FINISHED)
        .unwrap()
        .unwrap_err();
    let failures = error.downcast_ref::<HostShutdownErrors>().unwrap();
    assert!(failures.failure(HostShutdownStage::Service).is_some());
    assert!(failures.failure(HostShutdownStage::Ingress).is_some());
    drop(
        Arc::try_unwrap(host)
            .unwrap_or_else(|_| panic!("{STARTUP_ARM_HOST_OWNER_SURVIVED}")),
    );
    Host::open(HostConfig::new(directory.path())).unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn maintenance_start_failure_awaits_partial_worker_and_reopens_home() {
    let _worker_test = crate::maintenance::test_support::serialize_worker_tests().await;
    let directory = tempdir().unwrap();
    let home = directory.path().join(HOST_SHUTDOWN_HOME_NAME);
    let host = Host::open(HostConfig::new(&home)).unwrap();
    let socket = host.socket_path().to_path_buf();
    let owner = directory.path().join(STARTUP_SCANNER_OWNER);
    let (entered, entry) = std::sync::mpsc::sync_channel(RENDEZVOUS_CHANNEL_CAPACITY);
    let (release, released) = std::sync::mpsc::channel();
    let (dropped, scanner_drop) = tokio::sync::oneshot::channel();
    let scanner = StartupErrorScanner {
        entered: Some(entered),
        release: released,
        dropped: Some(dropped),
    };
    let plan = crate::maintenance::MaintenancePlan::single(owner, vec![Box::new(scanner)]);
    let serving = tokio::spawn(host.serve_until(std::future::pending(), move |serving| {
        serving.start_maintenance_plan(plan);
        entry
            .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
            .map_err(|_| anyhow::anyhow!(STARTUP_SCANNER_ENTRY_LOST))?;
        Err(anyhow::anyhow!(STARTUP_MAINTENANCE_ERROR))
    }));

    let replacement = wait_for_replacement_host(&home).await;
    assert!(!socket.exists());
    assert!(!serving.is_finished());
    release.send(()).unwrap();
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, scanner_drop)
        .await
        .expect(STARTUP_SCANNER_NOT_DROPPED)
        .expect(STARTUP_SCANNER_NOT_DROPPED);
    let error = tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, serving)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains(HOST_SHUTDOWN_MAINTENANCE_CONTEXT));
    assert!(format!("{error:#}").contains(STARTUP_MAINTENANCE_ERROR));
    drop(replacement);
}
