use super::ClientError;
use std::future::Future;
use tokio::time::Instant;

#[derive(Clone, Copy)]
pub(super) struct ClientDeadline {
    expires: Instant,
}

impl ClientDeadline {
    pub(super) fn instant(self) -> Instant { self.expires }
    pub(super) fn after(duration: std::time::Duration) -> Self {
        Self {
            expires: Instant::now() + duration,
        }
    }

    pub(super) async fn run<T>(self, future: impl Future<Output = T>) -> Result<T, ClientError> {
        tokio::time::timeout_at(self.expires, future)
            .await
            .map_err(|_| ClientError::Deadline)
    }

    #[cfg(test)]
    pub(super) fn test_after(duration: std::time::Duration) -> Self {
        Self::after(duration)
    }
}
