use super::constants::*;
use super::fixture::{MemoryConnection, connect_memory};
use crate::remote::{AuthorizedCommand, CommandPolicy, CommandRequest, MultiplexedSshConnection};

#[tokio::test]
async fn cancellation_after_exec_approval_kills_and_closes_exactly_once() {
    let fixture = connect_memory(false).await.unwrap();
    let command = authorized(HANG_PROGRAM);
    let execution = spawn_execution(&fixture, command);
    wait_until(&fixture, |fixture| {
        fixture.filesystem.live_commands() == EXPECTED_LIVE_COMMANDS
    })
    .await;

    execution.abort();
    assert!(execution.await.unwrap_err().is_cancelled());
    wait_for_cleanup(&fixture).await;
    assert_cleanup_once(&fixture);
    assert_connection_healthy(&fixture).await;
}

#[tokio::test]
async fn cancellation_during_output_collection_kills_and_closes_exactly_once() {
    let fixture = connect_memory(false).await.unwrap();
    let command = authorized(PARTIAL_PROGRAM);
    let execution = spawn_execution(&fixture, command);
    wait_until(&fixture, |fixture| {
        fixture.filesystem.command_outputs_started() == EXPECTED_LIVE_COMMANDS
    })
    .await;

    execution.abort();
    assert!(execution.await.unwrap_err().is_cancelled());
    wait_for_cleanup(&fixture).await;
    assert_cleanup_once(&fixture);
    assert_connection_healthy(&fixture).await;
}

#[tokio::test]
async fn direct_disconnect_waits_for_registered_command_cleanup() {
    let fixture = connect_memory(false).await.unwrap();
    let barrier = fixture.connection.cleanup.block_next_task_for_test();
    let execution = spawn_execution(&fixture, authorized(HANG_PROGRAM));
    wait_until(&fixture, |fixture| {
        fixture.filesystem.live_commands() == EXPECTED_LIVE_COMMANDS
    })
    .await;

    execution.abort();
    assert!(execution.await.unwrap_err().is_cancelled());
    let connection = fixture.connection.clone();
    let disconnect = tokio::spawn(async move { connection.disconnect().await });
    barrier.wait_entered().await;
    assert!(!disconnect.is_finished());

    barrier.release();
    disconnect.await.unwrap().unwrap();
    wait_for_cleanup(&fixture).await;
    assert_cleanup_once(&fixture);
}

fn authorized(program: &str) -> AuthorizedCommand {
    CommandPolicy::new([program.into()])
        .unwrap()
        .authorize(
            CommandRequest::new(program, vec![])
                .with_limits(COMMAND_OUTPUT_TEST_TIMEOUT, TEST_COMMAND_OUTPUT_LIMIT),
        )
        .unwrap()
}

fn spawn_execution(
    fixture: &MemoryConnection,
    command: AuthorizedCommand,
) -> tokio::task::JoinHandle<()> {
    let connection = fixture.connection.clone();
    tokio::spawn(async move {
        let result = connection.execute(&command).await;
        assert!(result.is_ok());
    })
}

async fn wait_for_cleanup(fixture: &MemoryConnection) {
    wait_until(fixture, |fixture| {
        fixture.filesystem.command_signals() == EXPECTED_COMMAND_CLEANUPS
            && fixture.filesystem.command_closes() == EXPECTED_COMMAND_CLEANUPS
            && fixture.filesystem.live_commands() == NO_LIVE_COMMANDS
    })
    .await;
}

fn assert_cleanup_once(fixture: &MemoryConnection) {
    assert_eq!(
        fixture.filesystem.command_signals(),
        EXPECTED_COMMAND_CLEANUPS
    );
    assert_eq!(
        fixture.filesystem.command_closes(),
        EXPECTED_COMMAND_CLEANUPS
    );
    assert_eq!(fixture.filesystem.live_commands(), NO_LIVE_COMMANDS);
}

async fn assert_connection_healthy(fixture: &MemoryConnection) {
    let output = fixture
        .connection
        .execute(&authorized(NORMAL_PROGRAM))
        .await;
    assert_eq!(output.unwrap().value.stdout, COMMAND_STDOUT);
}

async fn wait_until(fixture: &MemoryConnection, condition: impl Fn(&MemoryConnection) -> bool) {
    let settled = tokio::time::timeout(SFTP_LIFECYCLE_TIMEOUT, async {
        while !condition(fixture) {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(settled.is_ok());
}
