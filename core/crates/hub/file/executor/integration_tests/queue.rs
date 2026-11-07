use crate::{constants::*, faults::*, fixture::*};
use yolo_hub_file_executor::MAX_CONCURRENT_FILE_HELPERS;
use yolo_hub_file_reader_test_support::Fixture;
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn helper_capacity_is_shared_across_clones_and_queued_work_can_expire_without_spawn() {
    let fixtures = (0..MAX_CONCURRENT_FILE_HELPERS)
        .map(|_| Fixture::new())
        .collect::<Vec<_>>();
    let executor = fault_executor(&fixtures[0], HANG);
    let mut tasks = Vec::new();
    for fixture in &fixtures {
        let child = executor.clone();
        let request = inspect(fixture);
        tasks.push(tokio::spawn(async move {
            child.execute(request, deadline()).await
        }));
    }
    let mut proofs = Vec::new();
    for fixture in &fixtures {
        proofs.push(wait_proof(fixture).await);
    }
    let queued = Fixture::new();
    assert_eq!(
        executor
            .execute(
                inspect(&queued),
                tokio::time::Instant::now() + SHORT_DEADLINE
            )
            .await,
        Err(PublicError::Unreachable)
    );
    assert_eq!(
        std::fs::metadata(queued.workspace.join(PROOF_FILE))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotFound
    );
    executor.close();
    for task in tasks {
        assert_eq!(task.await.unwrap(), Err(PublicError::Unreachable));
    }
    for proof in proofs {
        wait_fault_retired(HANG, proof).await;
    }
}
