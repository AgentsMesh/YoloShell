use super::*;
use crate::remote::constants::{
    COMMAND_DATA_AFTER_EOF_MESSAGE, SUCCESS_EXIT_STATUS, UNEXPECTED_CHANNEL_REPLY_MESSAGE,
};

#[tokio::test]
async fn real_command_rejects_output_buffered_after_eof_and_keeps_connection_usable() {
    let fixture = connect_memory(false).await.unwrap();
    fixture
        .filesystem
        .fail_once(memory_fs::FailurePoint::ExecEofThenDataBeforeApproval);
    let policy = CommandPolicy::new([NORMAL_PROGRAM.into()]).unwrap();
    let command = policy
        .authorize(CommandRequest::new(NORMAL_PROGRAM, vec![]))
        .unwrap();

    assert!(matches!(
        fixture.connection.execute(&command).await,
        Err(RemoteError::Transport(message)) if message == COMMAND_DATA_AFTER_EOF_MESSAGE
    ));

    let healthy = fixture.connection.execute(&command).await.unwrap().value;
    assert_eq!(healthy.stdout, COMMAND_STDOUT);
    assert_eq!(healthy.stderr, COMMAND_STDERR);
    assert_eq!(healthy.exit_status, Some(SUCCESS_EXIT_STATUS));
    fixture.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn flow_control_before_exec_approval_is_rejected_without_exposing_early_output() {
    let fixture = connect_memory(false).await.unwrap();
    let failure = memory_fs::FailurePoint::ExecFlowControlBeforeApproval;
    fixture.filesystem.fail_once(failure);
    let command = hub_peer_fixture::command();
    assert!(matches!(
        fixture.connection.execute(&command).await,
        Err(RemoteError::Transport(message)) if message == UNEXPECTED_CHANNEL_REPLY_MESSAGE
    ));
    assert!(!fixture.filesystem.failure_is_armed(failure));
    hub_peer_fixture::assert_connection_healthy(&fixture).await;
    fixture
        .connection
        .cleanup
        .settle(cleanup_settlement_timeout(NORMAL_OPERATION_TIMEOUT))
        .await
        .unwrap();
}

#[tokio::test]
async fn data_without_exec_approval_times_out_without_exposing_unapproved_output() {
    let fixture = connect_memory(false).await.unwrap();
    let failure = memory_fs::FailurePoint::ExecDataWithoutApproval;
    fixture.filesystem.fail_once(failure);
    let command = CommandPolicy::new([NORMAL_PROGRAM.into()])
        .unwrap()
        .authorize(
            CommandRequest::new(NORMAL_PROGRAM, Vec::new())
                .with_limits(SHORT_OPERATION_TIMEOUT, TEST_COMMAND_OUTPUT_LIMIT),
        )
        .unwrap();
    let started = tokio::time::Instant::now();
    let output = fixture.connection.execute(&command).await.unwrap().value;
    // A completed timeout report is not evidence that exec was approved or exited.
    assert!(output.timed_out);
    assert_eq!(output.exit_status, None);
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    assert!(!output.truncated);
    assert!(!fixture.filesystem.failure_is_armed(failure));
    assert!(started.elapsed().as_millis() < MAX_OPERATION_ELAPSED_MS);
    hub_peer_fixture::assert_connection_healthy(&fixture).await;
    fixture
        .connection
        .cleanup
        .settle(cleanup_settlement_timeout(NORMAL_OPERATION_TIMEOUT))
        .await
        .unwrap();
}
