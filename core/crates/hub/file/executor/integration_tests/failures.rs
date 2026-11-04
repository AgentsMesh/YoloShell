use crate::{constants::*, faults::*, fixture::*};
use tokio::time::Instant;
use yolo_hub_file_executor_test_support::fault_lifecycle::rejects_while_alive;
use yolo_hub_file_reader_test_support::Fixture;
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn malformed_lost_duplicate_and_unsuccessful_replies_never_confirm_success() {
    for mode in [
        BAD_ID,
        BAD_ORIGIN,
        BAD_SCHEMA,
        OMIT_ROOT,
        DUPLICATE,
        TRUNCATED,
        EMPTY,
        OVERSIZED,
        FAIL_EXIT,
        CLOSED_CONTROL,
    ] {
        let fixture = Fixture::new();
        let executor = fault_executor(&fixture, mode);
        let limit = Instant::now() + SHORT_DEADLINE;
        let result = executor.execute(inspect(&fixture), limit).await;
        assert_eq!(result, Err(PublicError::Unreachable), "{mode}");
        if rejects_while_alive(mode) {
            assert!(Instant::now() < limit, "{mode}");
        }
        wait_fault_retired(mode, wait_proof(&fixture).await).await;
    }
}

#[tokio::test]
async fn stalled_filesystem_work_cannot_block_the_runtime_and_expires_without_a_survivor() {
    let fixture = Fixture::new();
    let executor = fault_executor(&fixture, HANG);
    let request = executor.execute(inspect(&fixture), Instant::now() + SHORT_DEADLINE);
    tokio::pin!(request);
    let mut clock = tokio::time::interval(POLL_INTERVAL);
    let mut ticks = 0;
    let result = loop {
        tokio::select! {
            result = &mut request => break result,
            _ = clock.tick() => ticks += 1,
        }
    };
    assert_eq!(result, Err(PublicError::Unreachable));
    assert!(ticks >= MINIMUM_HEARTBEATS);
    wait_fault_retired(HANG, wait_proof(&fixture).await).await;
}

#[tokio::test]
async fn cancellation_and_executor_shutdown_retire_exact_in_flight_children() {
    for close in [false, true] {
        let fixture = Fixture::new();
        let executor = fault_executor(&fixture, HANG);
        let running = executor.clone();
        let request = inspect(&fixture);
        let task = tokio::spawn(async move { running.execute(request, deadline()).await });
        let proof = wait_proof(&fixture).await;
        if close {
            executor.close();
            assert_eq!(task.await.unwrap(), Err(PublicError::Unreachable));
        } else {
            task.abort();
            assert!(task.await.unwrap_err().is_cancelled());
        }
        wait_fault_retired(HANG, proof).await;
    }
}
