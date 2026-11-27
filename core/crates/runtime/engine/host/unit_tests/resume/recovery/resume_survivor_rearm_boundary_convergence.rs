use crate::host::tests::*;
#[path = "../../../test_support/unit/socket/socket_namespace_certificate_test_support.rs"]
mod resume_namespace_tamper;

#[tokio::test]
async fn resume_survivor_rearm_reports_replacement_preparation_failure() {
    let directory = tempdir().unwrap();
    let host = Arc::new(unspawnable_resume_host(&directory));
    let (mut manifest, transaction, _) = dead_survivor_rearm_fixture(&host, directory.path());
    let catalog = Arc::clone(&host.catalog);
    let (locked, locked_ready) = std::sync::mpsc::channel();
    let (release, released) = std::sync::mpsc::channel();
    let blocker = std::thread::spawn(move || {
        let _guard = lock_recover(&catalog);
        locked.send(()).unwrap();
        released.recv_timeout(HOST_BLOCKING_TEST_TIMEOUT).unwrap();
    });
    locked_ready
        .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
        .unwrap();
    let (operations, mut observed) = tokio::sync::mpsc::unbounded_channel();
    *lock_recover(&host.catalog_test_probes.operations) = Some(operations);
    let task_host = Arc::clone(&host);
    let task_transaction = transaction.clone();
    let task = tokio::spawn(async move {
        task_host
            .rearm_dead_resume_terminal(&mut manifest, &task_transaction)
            .await
    });

    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, observed.recv())
        .await
        .unwrap()
        .unwrap();
    let tamper = resume_namespace_tamper::CorruptCertificate::new(
        host.socket_namespace.directory().unwrap(),
    );
    release.send(()).unwrap();
    blocker.join().unwrap();
    let result = task.await.unwrap();
    drop(tamper);

    assert_resume_failure_retains(&host, &transaction, result);
    assert!(!host.paths.worker_launch(transaction.terminal_id).exists());
}
