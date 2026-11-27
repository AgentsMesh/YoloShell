use super::*;

#[tokio::test]
async fn tracked_task_compensation_aborts_and_exact_handle_settles() {
    let (pending, task) =
        PendingBroker::tracked_task(async { std::future::pending::<Result<()>>().await });

    pending.compensate().unwrap();
    let settlement = task.settle().await.unwrap();

    assert!(matches!(settlement, TrackedTaskSettlement::Cancelled));
}
