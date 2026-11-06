async fn finalize_on_thread(
    state: Arc<WorkerState>,
    code: Option<i32>,
    signal: Option<i32>,
    drain_timeout: Duration,
) {
    tokio::task::spawn_blocking(move || {
        finalize_child_exit(&state, code, signal, drain_timeout);
    })
    .await
    .unwrap();
}
