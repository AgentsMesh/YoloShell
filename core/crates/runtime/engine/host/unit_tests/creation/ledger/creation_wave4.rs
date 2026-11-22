use crate::host::tests::*;
const CREATION_WAVE4_LOCK_HOLDER_AND_WAITER: usize = 2;
const CREATION_WAVE4_NAME: &str = "reservation-reread-race";

#[tokio::test]
async fn create_session_fails_closed_when_its_reserved_binding_disappears() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let (request, binding, ledger_path) =
        reserve_test_creation(&host, directory.path(), CREATION_WAVE4_NAME);
    let terminal_id = binding.result.terminal_id;
    let guard = host.session_locks.lock(terminal_id).await;
    let creating_host = Arc::clone(&host);
    let task = tokio::spawn(async move { creating_host.create_session(request).await });

    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, async {
        loop {
            let waiters = lock_recover(&host.session_locks.entries)
                .get(&terminal_id)
                .map(std::sync::Weak::strong_count)
                .unwrap_or_default();
            if waiters >= CREATION_WAVE4_LOCK_HOLDER_AND_WAITER {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();

    {
        let mut ledger = lock_recover(&host.creation_capabilities);
        let mut retired = ledger.clone();
        retired.bindings.clear();
        ledger.install(&ledger_path, retired).unwrap();
    }
    drop(guard);

    let error = task.await.unwrap().unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::INTERNAL);
    assert!(
        lock_recover(&host.creation_capabilities)
            .bindings
            .is_empty()
    );
    assert!(!host.paths.session_dir(terminal_id).exists());
}
