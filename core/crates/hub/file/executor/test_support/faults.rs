use crate::{constants::*, proof::Proof};
use std::path::PathBuf;
use tokio::time::{Instant, timeout_at};
use yolo_hub_file_executor::FileExecutor;
use yolo_hub_file_executor_test_support::fault_lifecycle::requires_interrupted_cleanup;
use yolo_hub_file_reader_test_support::Fixture;

pub fn fault_program(fixture: &Fixture, mode: &str) -> PathBuf {
    let source = PathBuf::from(std::env::var(FAULT_BINARY_ENV).unwrap())
        .canonicalize()
        .unwrap();
    let target = fixture.base.join(mode);
    std::fs::copy(source, &target).unwrap();
    target
}
pub fn fault_executor(fixture: &Fixture, mode: &str) -> FileExecutor {
    FileExecutor::new(fault_program(fixture, mode)).unwrap()
}
pub async fn wait_proof(fixture: &Fixture) -> Proof {
    wait_proof_at(&fixture.workspace).await
}
pub async fn wait_proof_at(directory: &std::path::Path) -> Proof {
    timeout_at(Instant::now() + TEST_DEADLINE, async {
        loop {
            if let Ok(bytes) = std::fs::read(directory.join(PROOF_FILE)) {
                if let Ok(proof) = serde_json::from_slice(&bytes) {
                    return proof;
                }
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    })
    .await
    .expect("helper did not publish its process proof")
}
pub async fn wait_gone(proof: Proof) {
    timeout_at(Instant::now() + TEST_DEADLINE, async {
        while !proof.gone() {
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    })
    .await
    .expect("exact helper process remained alive");
}

pub async fn wait_fault_retired(mode: &str, proof: Proof) {
    wait_gone(proof).await;
    if requires_interrupted_cleanup(mode) {
        // The fixture captured this PID/start pair before its fault response.
        // Only modes that remain alive for forced retirement discard profiles.
        yolo_coverage_test_support::discard_aborted_process_profile(proof.pid)
            .expect(ABORTED_PROFILE_CLEANUP);
    }
}
