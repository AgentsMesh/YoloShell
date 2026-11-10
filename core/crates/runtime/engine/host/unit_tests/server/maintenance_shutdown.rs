use crate::host::tests::*;
use super::*;

const HOST_SHUTDOWN_SCANNER_LABEL: &str = "host-shutdown-test";
pub(super) const HOST_SHUTDOWN_HOME_NAME: &str = "home";
const HOST_SHUTDOWN_OWNER_NAME: &str = "maintenance-owner.lock";

#[tokio::test(flavor = "current_thread")]
async fn host_replacement_survives_bounded_maintenance_shutdown() {
    let directory = tempdir().unwrap();
    let home = directory.path().join(HOST_SHUTDOWN_HOME_NAME);
    let host = Host::open(HostConfig::new(&home)).unwrap();
    let socket = host.socket_path().to_path_buf();
    let owner = directory.path().join(HOST_SHUTDOWN_OWNER_NAME);
    let (mut barrier, scanner) =
        crate::maintenance::test_support::ScannerBarrier::new(HOST_SHUTDOWN_SCANNER_LABEL);
    let plan = crate::maintenance::MaintenancePlan::single(owner, vec![Box::new(scanner)]);
    let (shutdown, shutdown_requested) = tokio::sync::oneshot::channel();
    let serve = tokio::spawn(host.serve_until_with_maintenance(
        async move {
            let _ = shutdown_requested.await;
        },
        plan,
    ));
    wait_for_host_socket(&socket).await;
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, barrier.wait_entered())
        .await
        .unwrap();
    let failsafe = barrier.arm_failsafe(HOST_BLOCKING_TEST_TIMEOUT);

    shutdown.send(()).unwrap();
    let replacement = wait_for_replacement_host(&home).await;

    let error = tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, serve)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains(HOST_SHUTDOWN_MAINTENANCE_CONTEXT)
    );

    barrier.release();
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, barrier.wait_dropped())
        .await
        .unwrap();
    crate::maintenance::test_support::settle_failsafe(failsafe)
        .await
        .unwrap();
    assert!(!barrier.was_forced());
    drop(replacement);
}

pub(super) async fn wait_for_replacement_host(home: &Path) -> Host {
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, async {
        loop {
            match Host::open(HostConfig::new(home)) {
                Ok(host) => break host,
                Err(_) => tokio::time::sleep(STARTUP_SOCKET_POLL_INTERVAL).await,
            }
        }
    })
    .await
    .unwrap()
}
