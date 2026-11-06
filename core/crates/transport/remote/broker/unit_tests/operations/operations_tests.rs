use super::super::*;
use super::support::*;
use crate::remote::{CommandPolicy, FileReadPolicy, RemoteError, RuntimeRole};
use std::path::PathBuf;
use std::sync::Arc;

const INVALID_COMMAND_TIMEOUT_MILLIS: u64 = 0;

fn setup() -> (Arc<FakeConnection>, BrokerConnectionPool, BrokerConfig) {
    let connection = FakeConnection::new([]);
    let factory = FakeFactory::new(
        vec![Arc::clone(&connection)],
        vec![ConnectStep::Connection(0)],
    );
    let config = test_config();
    let pool = BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory,
    );
    (connection, pool, config)
}

#[tokio::test]
async fn command_file_and_attachment_channels_run_in_parallel() {
    let (connection, pool, config) = setup();
    let command = BrokerCommandRequest::standard(ALLOWED_PROGRAM, vec!["status".into()]);
    let file = BrokerFileRequest {
        path: PathBuf::from(READ_PATH),
        offset: 0,
        length: 4,
    };
    let (command, file, runtime) = tokio::join!(
        pool.run_command(&config.command_policy, command),
        pool.read_file(&config.file_policy, file),
        pool.open_runtime(RuntimeRole::Attachment),
    );
    let BrokerResult::Command {
        connection_id: command_id,
        output,
    } = command.unwrap()
    else {
        panic!("wrong command result")
    };
    let BrokerResult::FileRead {
        connection_id: file_id,
        result,
    } = file.unwrap()
    else {
        panic!("wrong file result")
    };
    assert_eq!(command_id, connection.id());
    assert_eq!(file_id, connection.id());
    assert_eq!(output.stdout, b"ok");
    assert_eq!(result.data, b"data");
    assert!(runtime.is_ok());
    assert_eq!(connection.commands(), 1);
    assert_eq!(connection.files(), 1);
}

#[tokio::test]
async fn working_directory_uses_a_separate_channel_on_the_shared_connection() {
    let (connection, pool, _) = setup();
    let result = pool.working_directory().await.unwrap();
    let BrokerResult::WorkingDirectory {
        connection_id,
        path,
    } = result
    else {
        panic!("wrong working directory result")
    };
    assert_eq!(connection_id, connection.id());
    assert_eq!(path, PathBuf::from(READ_PATH));
    assert_eq!(connection.files(), 0);
}

#[tokio::test]
async fn policy_rejections_do_not_open_an_ssh_connection() {
    let (_, pool, config) = setup();
    let denied = BrokerCommandRequest::standard("/usr/bin/rm", vec!["target".into()]);
    assert!(matches!(
        pool.run_command(&config.command_policy, denied).await,
        Err(RemoteError::CommandDenied(_))
    ));
    let outside = BrokerFileRequest {
        path: PathBuf::from("/etc/passwd"),
        offset: 0,
        length: 4,
    };
    assert!(matches!(
        pool.read_file(&config.file_policy, outside).await,
        Err(RemoteError::PathDenied(_))
    ));
    assert_eq!(pool.status().await.connection_id, None);
}

#[tokio::test]
async fn invalid_limits_and_empty_policies_fail_closed() {
    let (_, pool, config) = setup();
    let mut command = BrokerCommandRequest::standard(ALLOWED_PROGRAM, vec![]);
    command.timeout_millis = INVALID_COMMAND_TIMEOUT_MILLIS;
    assert!(matches!(
        pool.run_command(&config.command_policy, command).await,
        Err(RemoteError::InvalidCommand(_))
    ));
    let file = BrokerFileRequest {
        path: PathBuf::from(READ_PATH),
        offset: u64::MAX,
        length: 2,
    };
    assert!(matches!(
        pool.read_file(&config.file_policy, file).await,
        Err(RemoteError::PathDenied(_))
    ));
    assert!(CommandPolicy::new(Vec::<String>::new()).is_err());
    assert!(FileReadPolicy::standard(Vec::new()).is_err());
}
