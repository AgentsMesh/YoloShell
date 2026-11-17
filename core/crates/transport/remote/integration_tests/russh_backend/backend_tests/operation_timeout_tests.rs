use super::constants::*;
use super::fixture::connect_memory_with_timeout;
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use crate::remote::*;
use std::path::PathBuf;
use std::time::Instant;

#[tokio::test]
async fn runtime_channel_open_has_a_deadline() {
    let (fixture, filesystem) = new_fixture().await;
    let runtime = RemoteRuntimeSpec::new(MEMORY_RUNTIME, MEMORY_HOME).unwrap();
    filesystem.fail_once(FailurePoint::ChannelOpen);
    let started = Instant::now();
    assert!(matches!(
        fixture
            .connection
            .open_runtime(&RuntimeChannelRequest::control(runtime))
            .await,
        Err(RemoteError::Transport(_))
    ));
    assert_bounded(started);
    finish_hostile_disconnect(&fixture).await;
}

#[tokio::test]
async fn command_execution_and_cleanup_have_a_deadline() {
    let (fixture, filesystem) = new_fixture().await;
    filesystem.fail_once(FailurePoint::ChannelOpen);
    assert!(matches!(
        fixture
            .connection
            .execute(&authorized_command(NORMAL_PROGRAM))
            .await,
        Err(RemoteError::Transport(_))
    ));
    finish_hostile_disconnect(&fixture).await;

    let (fixture, filesystem) = new_fixture().await;
    filesystem.fail_once(FailurePoint::ExecReject);
    assert!(
        fixture
            .connection
            .execute(&authorized_command(NORMAL_PROGRAM))
            .await
            .is_err()
    );
    filesystem.fail_once(FailurePoint::ExecReject);
    let runtime = RemoteRuntimeSpec::new(MEMORY_RUNTIME, MEMORY_HOME).unwrap();
    assert!(
        fixture
            .connection
            .open_runtime(&RuntimeChannelRequest::attachment(runtime))
            .await
            .is_err()
    );
    let started = Instant::now();
    let output = fixture
        .connection
        .execute(&authorized_command(HANG_PROGRAM))
        .await
        .unwrap()
        .value;
    assert!(output.timed_out);
    assert_bounded(started);
    let output = fixture
        .connection
        .execute(&authorized_command(NORMAL_PROGRAM))
        .await
        .unwrap()
        .value;
    assert_eq!(output.stdout, COMMAND_STDOUT);
}

#[tokio::test]
async fn canonicalize_and_read_have_independent_deadlines() {
    let (fixture, filesystem) = new_fixture().await;
    filesystem.fail_once(FailurePoint::SftpCanonicalize);
    let started = Instant::now();
    assert!(matches!(
        fixture
            .connection
            .canonicalize(std::path::Path::new(MEMORY_FILE))
            .await,
        Err(RemoteError::Transport(_))
    ));
    assert_bounded(started);

    let policy = FileReadPolicy::standard(vec![PathBuf::from("/srv/project")]).unwrap();
    let read = policy
        .authorize(FileReadRequest::new(MEMORY_FILE, 0, 4), MEMORY_FILE)
        .unwrap();
    filesystem.fail_once(FailurePoint::SftpRead);
    let started = Instant::now();
    assert!(matches!(
        fixture.connection.read_file(&read).await,
        Err(RemoteError::Transport(_))
    ));
    assert_bounded(started);
    let output = fixture
        .connection
        .execute(&authorized_command(NORMAL_PROGRAM))
        .await
        .unwrap()
        .value;
    assert_eq!(output.stdout, COMMAND_STDOUT);
}

async fn new_fixture() -> (super::fixture::MemoryConnection, MemoryFilesystem) {
    let filesystem = MemoryFilesystem::default();
    let fixture = connect_memory_with_timeout(false, filesystem.clone(), SHORT_OPERATION_TIMEOUT)
        .await
        .unwrap();
    (fixture, filesystem)
}

fn authorized_command(program: &str) -> AuthorizedCommand {
    CommandPolicy::new([program.into()])
        .unwrap()
        .authorize(
            CommandRequest::new(program, vec![])
                .with_limits(SHORT_OPERATION_TIMEOUT, TEST_COMMAND_OUTPUT_LIMIT),
        )
        .unwrap()
}

fn assert_bounded(started: Instant) {
    assert!(started.elapsed().as_millis() < MAX_OPERATION_ELAPSED_MS);
}

async fn finish_hostile_disconnect(fixture: &super::fixture::MemoryConnection) {
    tokio::time::timeout(TEST_INACTIVITY_TIMEOUT, async {
        while !fixture.connection.session.is_closed() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    fixture.filesystem.release_channel_open();
    tokio::time::timeout(
        TEST_INACTIVITY_TIMEOUT,
        fixture.filesystem.wait_connection_closed(),
    )
    .await
    .unwrap();
}
