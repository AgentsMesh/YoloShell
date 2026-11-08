//! Host operation deadlines. A deadline is a local budget for one operation;
//! it is not propagated as an end-to-end route deadline.

use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub struct HostOperationDeadline {
    expires_at: tokio::time::Instant,
    #[cfg(any(test, feature = "yolo-test-support"))]
    expire_at_commit: bool,
}

impl HostOperationDeadline {
    pub fn after(timeout: Duration) -> Self {
        Self {
            expires_at: tokio::time::Instant::now() + timeout,
            #[cfg(any(test, feature = "yolo-test-support"))]
            expire_at_commit: false,
        }
    }

    pub fn reserving(self, settlement: Duration) -> Self {
        let now = tokio::time::Instant::now();
        let expires_at = self
            .expires_at
            .checked_sub(settlement)
            .unwrap_or(now)
            .max(now);
        Self {
            expires_at,
            #[cfg(any(test, feature = "yolo-test-support"))]
            expire_at_commit: self.expire_at_commit,
        }
    }

    #[cfg(any(test, feature = "yolo-test-support"))]
    pub fn expiring_at_commit(timeout: Duration) -> Self {
        Self {
            expires_at: tokio::time::Instant::now() + timeout,
            expire_at_commit: true,
        }
    }

    pub fn is_elapsed(self) -> bool {
        tokio::time::Instant::now() >= self.expires_at
    }

    pub fn is_commit_elapsed(self) -> bool {
        #[cfg(any(test, feature = "yolo-test-support"))]
        if self.expire_at_commit {
            return true;
        }
        self.is_elapsed()
    }

    pub fn process_instant(self) -> std::time::Instant {
        self.expires_at.into_std()
    }

    pub async fn timeout<F>(self, future: F) -> Option<F::Output>
    where
        F: std::future::Future,
    {
        if self.is_elapsed() {
            return None;
        }
        tokio::time::timeout_at(self.expires_at, future).await.ok()
    }

    pub async fn wait_for_poll(self, interval: Duration) {
        if self.is_elapsed() {
            return;
        }
        let wake_at = (tokio::time::Instant::now() + interval).min(self.expires_at);
        tokio::time::sleep_until(wake_at).await;
    }
}
