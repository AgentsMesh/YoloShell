use super::*;
use std::future::{pending, ready};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const CHILD_EXECUTABLE: &str = "/bin/sleep";
const CHILD_DURATION: &str = "1";
const CHILD_EXIT_TIMEOUT: Duration = Duration::from_secs(3);
const CHILD_EXIT_POLL: Duration = Duration::from_millis(10);
const NATURAL_CUSTODY_PROBE_DELAY: Duration = Duration::from_millis(100);
const BOUNDARY_TIMEOUT: Duration = Duration::from_millis(20);
const TRANSMISSION_COMPLETED_UNEXPECTEDLY: &str =
    "injected descriptor transmission unexpectedly completed";
const POST_BOUNDARY_PROTOCOL_FAILURE: &str = "injected guardian ready validation failure";

#[tokio::test]
async fn cancellation_before_descriptor_transmission_terminates_guardian() {
    let (mut settlement, process_id) = spawn_test_guardian();
    let transmission = after_publication_boundary(&mut settlement, pending::<()>());

    drop(transmission);
    drop(settlement);

    wait_for_termination();
    wait_until_not_alive(process_id);
}

#[tokio::test]
async fn cancellation_during_descriptor_transmission_retains_guardian() {
    let (mut settlement, process_id) = spawn_test_guardian();
    let mut transmission = Box::pin(after_publication_boundary(&mut settlement, pending::<()>()));
    tokio::select! {
        biased;
        _ = &mut transmission => panic!("{TRANSMISSION_COMPLETED_UNEXPECTEDLY}"),
        _ = tokio::task::yield_now() => {}
    }

    drop(transmission);
    drop(settlement);

    assert_natural_custody(process_id);
}

#[tokio::test]
async fn protocol_failure_after_descriptor_transmission_retains_guardian() {
    let (mut settlement, process_id) = spawn_test_guardian();
    after_publication_boundary(&mut settlement, ready(())).await;
    let validation = Err::<(), _>(anyhow::anyhow!(POST_BOUNDARY_PROTOCOL_FAILURE));

    assert!(validation.is_err());
    drop(settlement);

    assert_natural_custody(process_id);
}

#[tokio::test]
async fn timeout_after_descriptor_transmission_retains_guardian() {
    let (mut settlement, process_id) = spawn_test_guardian();
    let exchange = after_publication_boundary(&mut settlement, pending::<()>());
    let result = tokio::time::timeout(BOUNDARY_TIMEOUT, exchange).await;

    assert!(result.is_err());
    drop(settlement);

    assert_natural_custody(process_id);
}

#[tokio::test]
async fn successful_ready_uses_reserved_natural_reaper_custody() {
    let (mut settlement, process_id) = spawn_test_guardian();
    after_publication_boundary(&mut settlement, ready(())).await;

    settlement.retain_naturally().unwrap();

    assert_natural_custody(process_id);
}

fn spawn_test_guardian() -> (GuardianSpawnSettlement, u32) {
    let mut command = Command::new(CHILD_EXECUTABLE);
    command
        .arg(CHILD_DURATION)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    crate::process::configure_detached_command(&mut command);
    let settlement = GuardianSpawnSettlement::spawn(&mut command).unwrap();
    let process_id = settlement.child().unwrap().id();
    (settlement, process_id)
}

fn assert_natural_custody(process_id: u32) {
    wait_for_termination();
    std::thread::sleep(NATURAL_CUSTODY_PROBE_DELAY);
    assert!(crate::process::is_process_alive(process_id));
    wait_until_not_alive(process_id);
}

fn wait_for_termination() {
    crate::process::wait_for_detached_termination_delivery(Instant::now() + CHILD_EXIT_TIMEOUT)
        .unwrap();
}

fn wait_until_not_alive(process_id: u32) {
    let deadline = Instant::now() + CHILD_EXIT_TIMEOUT;
    while crate::process::is_process_alive(process_id) {
        assert!(Instant::now() < deadline);
        std::thread::sleep(CHILD_EXIT_POLL);
    }
}
