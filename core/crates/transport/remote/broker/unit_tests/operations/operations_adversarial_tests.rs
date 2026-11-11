use super::super::*;
use super::support::*;
use super::support_operations::{OperationConnection, OperationFault};
use crate::protocol::ProtocolError;
use crate::remote::{OPERATION_TIMEOUT_MESSAGE, RemoteError};
use std::path::PathBuf;

fn build_pool(fault: OperationFault) -> (BrokerConnectionPool, BrokerConfig) {
    let connection = OperationConnection::new(fault);
    let factory = FakeFactory::new(vec![connection], vec![ConnectStep::Connection(0)]);
    let config = test_config();
    let pool = BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory,
    );
    (pool, config)
}

#[tokio::test]
async fn working_directory_rejects_identity_and_unsafe_path_results() {
    for fault in [
        OperationFault::CanonicalConnectionMismatch,
        OperationFault::CanonicalPurposeMismatch,
    ] {
        let (pool, _) = build_pool(fault);
        assert!(matches!(
            pool.working_directory().await,
            Err(RemoteError::ChannelMismatch)
        ));
    }
    let (pool, _) = build_pool(OperationFault::CanonicalUnsafe);
    assert!(matches!(
        pool.working_directory().await,
        Err(RemoteError::PathDenied(_))
    ));
}

#[tokio::test]
async fn working_directory_transport_loss_invalidates_and_reconnects() {
    let first = OperationConnection::new(OperationFault::CanonicalTransport);
    let second = OperationConnection::new(OperationFault::None);
    let second_id = second.id();
    let factory = FakeFactory::new(
        vec![first, second],
        vec![ConnectStep::Connection(0), ConnectStep::Connection(1)],
    );
    let config = test_config();
    let pool = BrokerConnectionPool::new(
        HOST_ID,
        config.revision(),
        config.runtime().clone(),
        factory,
    );
    assert!(pool.working_directory().await.is_err());
    assert!(pool.status().await.connection_id.is_none());
    let BrokerResult::WorkingDirectory { connection_id, .. } =
        pool.working_directory().await.unwrap()
    else {
        panic!("wrong working directory result")
    };
    assert_eq!(connection_id, second_id);
}

fn command(cwd: bool) -> BrokerCommandRequest {
    let mut request = BrokerCommandRequest::standard(ALLOWED_PROGRAM, Vec::new());
    request.cwd = cwd.then(|| PathBuf::from(READ_ROOT));
    request
}

fn file() -> BrokerFileRequest {
    BrokerFileRequest {
        path: READ_PATH.into(),
        offset: 0,
        length: 4,
    }
}

#[tokio::test]
async fn command_transport_error_invalidates_generation_but_channel_rejection_does_not() {
    let (pool, config) = build_pool(OperationFault::CommandTransport);
    assert!(
        pool.run_command(&config.command_policy, command(true))
            .await
            .is_err()
    );
    assert!(pool.status().await.connection_id.is_none());

    let (pool, config) = build_pool(OperationFault::CommandRejected);
    assert!(matches!(
        pool.run_command(&config.command_policy, command(false))
            .await,
        Err(RemoteError::ChannelRejected)
    ));
    assert!(pool.status().await.connection_id.is_some());
}

#[tokio::test]
async fn command_identity_and_output_are_verified() {
    for fault in [
        OperationFault::CommandConnectionMismatch,
        OperationFault::CommandPurposeMismatch,
    ] {
        let (pool, config) = build_pool(fault);
        assert!(matches!(
            pool.run_command(&config.command_policy, command(false))
                .await,
            Err(RemoteError::ChannelMismatch)
        ));
    }
    let (pool, config) = build_pool(OperationFault::CommandInvalidOutput);
    assert!(matches!(
        pool.run_command(&config.command_policy, command(false))
            .await,
        Err(RemoteError::LimitExceeded)
    ));
}

#[tokio::test]
async fn canonicalization_errors_are_contained_before_file_read() {
    for (fault, lost) in [
        (OperationFault::CanonicalTransport, true),
        (OperationFault::CanonicalRejected, false),
    ] {
        let (pool, config) = build_pool(fault);
        assert!(pool.read_file(&config.file_policy, file()).await.is_err());
        assert_eq!(pool.status().await.connection_id.is_none(), lost);
    }
    for fault in [
        OperationFault::CanonicalConnectionMismatch,
        OperationFault::CanonicalPurposeMismatch,
        OperationFault::CanonicalOutside,
    ] {
        let (pool, config) = build_pool(fault);
        assert!(pool.read_file(&config.file_policy, file()).await.is_err());
    }
}

#[tokio::test]
async fn file_result_errors_are_verified_and_loss_invalidates_transport() {
    for (fault, lost) in [
        (OperationFault::ReadTransport, true),
        (OperationFault::ReadLifecycleSettlement, true),
        (OperationFault::ReadRejected, false),
    ] {
        let (pool, config) = build_pool(fault);
        assert!(pool.read_file(&config.file_policy, file()).await.is_err());
        assert_eq!(pool.status().await.connection_id.is_none(), lost);
    }
    for fault in [
        OperationFault::ReadConnectionMismatch,
        OperationFault::ReadPurposeMismatch,
        OperationFault::ReadInvalidOutput,
    ] {
        let (pool, config) = build_pool(fault);
        assert!(pool.read_file(&config.file_policy, file()).await.is_err());
    }
}

#[test]
fn connection_loss_classifier_is_fail_closed() {
    assert!(super::super::connection_runtime::connection_lost(
        &RemoteError::Protocol(ProtocolError::EndOfStream)
    ));
    assert!(super::super::connection_runtime::connection_lost(
        &RemoteError::Protocol(ProtocolError::Io(std::io::Error::from(
            std::io::ErrorKind::BrokenPipe
        )))
    ));
    assert!(!super::super::connection_runtime::connection_lost(
        &RemoteError::ChannelRejected
    ));
    assert!(super::super::connection_runtime::connection_lost(
        &RemoteError::LifecycleSettlement {
            primary: Box::new(RemoteError::PathDenied(PathBuf::from(READ_PATH))),
            settlement: Box::new(RemoteError::transport(OPERATION_TIMEOUT_MESSAGE)),
        }
    ));
    assert!(super::super::connection_runtime::connection_lost(
        &RemoteError::LifecycleSettlement {
            primary: Box::new(RemoteError::LifecycleSettlement {
                primary: Box::new(RemoteError::PathDenied(PathBuf::from(READ_PATH))),
                settlement: Box::new(RemoteError::transport(OPERATION_TIMEOUT_MESSAGE)),
            }),
            settlement: Box::new(RemoteError::ChannelRejected),
        }
    ));
}
