use super::*;

#[tokio::test(start_paused = true)]
async fn total_deadline_bounds_repeated_post_bootstrap_connection_loss() {
    let _clock = crate::remote_broker::tests::runtime_client::DrivenClock::new();
    let losses =
        std::iter::repeat(OpenStep::TransportFailure).take(LOSS_STEPS_BEYOND_RUNTIME_DEADLINE);
    let connection = FakeConnection::new(losses);
    let factory = Arc::new(RepeatingFactory {
        connection,
        calls: AtomicUsize::new(NO_CONNECT_CALLS),
    });
    let shared = pool(factory.clone());
    let task_pool = Arc::clone(&shared);
    let task = tokio::spawn(async move { task_pool.open_runtime(RuntimeRole::Control).await });
    while factory.calls.load(Ordering::Relaxed) == NO_CONNECT_CALLS {
        tokio::task::yield_now().await;
    }

    let advance_budget = RUNTIME_OPEN_TIMEOUT + MAX_RECONNECT_DELAY;
    let mut advanced = std::time::Duration::ZERO;
    while advanced < advance_budget {
        tokio::time::advance(MAX_RECONNECT_DELAY).await;
        advanced += MAX_RECONNECT_DELAY;
    }
    let result = task.await.unwrap();
    let Err(error) = result else {
        panic!("{RUNTIME_FAILURE_REQUIRED}")
    };
    assert!(matches!(
        error,
        RemoteError::RuntimeDeployment(message) if message == RUNTIME_OPEN_TIMEOUT_MESSAGE
    ));
    assert!(factory.calls.load(Ordering::Relaxed) > EXPECTED_FIRST_CALL);
    assert!(shared.status().await.connection_id.is_none());
    assert!(RUNTIME_OPEN_TIMEOUT < advanced);
}
