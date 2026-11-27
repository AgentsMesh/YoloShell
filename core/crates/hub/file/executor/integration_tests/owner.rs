use crate::{constants::*, direct::DirectChild, faults::*, fixture::*};
use std::{
    io::Write,
    process::{Command, Stdio},
};
use yolo_hub_file_helper_protocol::{
    HELPER_EXIT_EXPIRED, HELPER_EXIT_FAILURE, MAX_HELPER_LIFETIME,
};
use yolo_hub_file_reader_test_support::Fixture;
use yolo_process::ScopedWaitableChild;

#[tokio::test]
async fn owner_channel_loss_and_invalid_owner_data_retire_even_a_blocked_helper() {
    for send_data in [false, true] {
        let fixture = Fixture::new();
        let mut child =
            DirectChild::spawn(&fault_program(&fixture, HANG), &inspect(&fixture)).await;
        let proof = wait_proof(&fixture).await;
        if send_data {
            child
                .owner
                .as_mut()
                .unwrap()
                .write_all(INVALID_OWNER_DATA)
                .unwrap();
        } else {
            child.owner.take();
        }
        assert_eq!(
            child.exited(deadline()).await.code(),
            Some(HELPER_EXIT_EXPIRED)
        );
        wait_gone(proof).await;
    }
}

#[tokio::test]
async fn watchdog_retires_blocked_work_even_while_its_owner_remains_alive() {
    let fixture = Fixture::new();
    let mut child = DirectChild::spawn(&fault_program(&fixture, HANG), &inspect(&fixture)).await;
    let proof = wait_proof(&fixture).await;
    let limit = tokio::time::Instant::now() + MAX_HELPER_LIFETIME + TEST_DEADLINE;
    assert_eq!(child.exited(limit).await.code(), Some(HELPER_EXIT_EXPIRED));
    assert!(child.owner.is_some());
    wait_gone(proof).await;
}

#[tokio::test]
async fn killing_the_executor_process_does_not_leave_its_detached_helper_alive() {
    let fixture = Fixture::new();
    fault_program(&fixture, HANG);
    let mut launcher =
        DirectChild::spawn(&fault_program(&fixture, LAUNCHER), &inspect(&fixture)).await;
    let proof = wait_proof(&fixture).await;
    assert_ne!(launcher.child.as_ref().unwrap().process_id(), proof.pid);
    launcher.terminate().await;
    wait_fault_retired(HANG, proof).await;
}

#[tokio::test]
async fn a_standalone_invocation_without_private_channels_fails_before_any_file_work() {
    let mut command = Command::new(binary());
    command
        .args(yolo_launch_contract::ProcessRole::FileHelper.arguments())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child =
        ScopedWaitableChild::spawn_detached_before_deadline(&mut command, deadline().into_std())
            .unwrap();
    drop(command);
    let status = tokio::time::timeout_at(deadline(), async {
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status;
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    })
    .await
    .unwrap();
    child.retire_reaped().unwrap();
    assert_eq!(status.code(), Some(i32::from(HELPER_EXIT_FAILURE)));
}

#[tokio::test]
async fn version_queries_require_no_private_channels_or_filesystem_authority() {
    use std::io::Read;
    let mut command = Command::new(binary());
    command
        .args(yolo_launch_contract::ProcessRole::FileHelper.arguments())
        .arg(VERSION_ARGUMENT)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child =
        ScopedWaitableChild::spawn_detached_before_deadline(&mut command, deadline().into_std())
            .unwrap();
    drop(command);
    let mut output = child.take_stdout().unwrap();
    child
        .wait_for_clean_exit(deadline().into_std())
        .await
        .unwrap();
    let mut text = String::new();
    output.read_to_string(&mut text).unwrap();
    assert_eq!(
        text.trim(),
        format!(
            "{} {}",
            yolo_launch_contract::ProcessRole::FileHelper.display_name(),
            env!("CARGO_PKG_VERSION")
        )
    );
}
