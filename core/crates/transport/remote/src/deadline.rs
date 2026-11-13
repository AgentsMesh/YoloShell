use super::RemoteError;
use super::constants::{OPERATION_TIMEOUT_MESSAGE, SSH_OPERATION_TIMEOUT};
use std::future::Future;
use tokio::time::Instant;

#[derive(Clone, Copy)]
pub struct RemoteDeadline {
    expires: Instant,
}

impl RemoteDeadline {
    pub fn instant(self) -> Instant { self.expires }
    pub fn operation() -> Self {
        Self {
            expires: Instant::now() + SSH_OPERATION_TIMEOUT,
        }
    }

    pub async fn run<T>(
        self,
        future: impl Future<Output = Result<T, RemoteError>>,
    ) -> Result<T, RemoteError> {
        tokio::time::timeout_at(self.expires, future)
            .await
            .map_err(|_| RemoteError::transport(OPERATION_TIMEOUT_MESSAGE))?
    }
}
