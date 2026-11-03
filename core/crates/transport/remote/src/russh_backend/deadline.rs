use super::super::constants::OPERATION_TIMEOUT_MESSAGE;
use super::{RemoteError, ssh_error};
use std::future::Future;
use std::time::Duration;

pub(super) async fn operation<T, E>(
    limit: Duration,
    future: impl Future<Output = Result<T, E>>,
) -> Result<T, RemoteError>
where
    E: std::fmt::Display,
{
    wait(limit, future).await?.map_err(ssh_error)
}

pub(super) async fn wait<T>(
    limit: Duration,
    future: impl Future<Output = T>,
) -> Result<T, RemoteError> {
    tokio::time::timeout(limit, future)
        .await
        .map_err(|_| RemoteError::transport(OPERATION_TIMEOUT_MESSAGE))
}

pub(super) async fn remote_operation<T>(
    limit: Duration,
    future: impl Future<Output = Result<T, RemoteError>>,
) -> Result<T, RemoteError> {
    wait(limit, future).await?
}
