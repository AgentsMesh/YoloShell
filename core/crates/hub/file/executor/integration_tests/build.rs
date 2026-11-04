use crate::{constants::*, faults::*, fixture::*};
use yolo_hub_file_executor::FileExecutor;
use yolo_hub_file_executor_test_support::fault_lifecycle::rejects_while_alive;
use yolo_hub_file_reader_test_support::Fixture;
use yolo_hub_identity::HubId;
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn build_evidence_comes_from_the_actual_configured_helper_process() {
    let image = executor()
        .build_image(HubId::new(), deadline())
        .await
        .unwrap();
    let bytes = std::fs::read(binary()).unwrap();
    assert_eq!(image.sha256(), yolo_crypto::sha256_hex(&bytes));
    assert_eq!(image.size_bytes(), bytes.len() as u64);
    let fixture = Fixture::new();
    let alternate = fault_program(&fixture, BUILD);
    let alternate_bytes = std::fs::read(&alternate).unwrap();
    let other = FileExecutor::new(alternate)
        .unwrap()
        .build_image(HubId::new(), deadline())
        .await
        .unwrap();
    assert_eq!(other.sha256(), yolo_crypto::sha256_hex(&alternate_bytes));
    assert_eq!(other.size_bytes(), alternate_bytes.len() as u64);
    assert_ne!(image, other);
    wait_gone(wait_proof_at(&fixture.base).await).await;
}

#[tokio::test]
async fn build_queries_keep_correlation_eof_successful_exit_and_child_custody_checks() {
    for mode in [
        BAD_ID,
        BAD_ORIGIN,
        BAD_SCHEMA,
        DUPLICATE,
        FAIL_EXIT,
        CLOSED_CONTROL,
        HANG,
    ] {
        let fixture = Fixture::new();
        let executor = fault_executor(&fixture, mode);
        let limit = tokio::time::Instant::now() + SHORT_DEADLINE;
        assert_eq!(
            executor.build_image(HubId::new(), limit).await,
            Err(PublicError::Unreachable),
            "{mode}"
        );
        if rejects_while_alive(mode) {
            assert!(tokio::time::Instant::now() < limit, "{mode}");
        }
        wait_fault_retired(mode, wait_proof_at(&fixture.base).await).await;
    }
}

#[tokio::test]
async fn cancelled_build_query_retires_its_exact_helper() {
    let fixture = Fixture::new();
    let executor = fault_executor(&fixture, HANG);
    let task = tokio::spawn(async move { executor.build_image(HubId::new(), deadline()).await });
    let proof = wait_proof_at(&fixture.base).await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    wait_fault_retired(HANG, proof).await;
}
