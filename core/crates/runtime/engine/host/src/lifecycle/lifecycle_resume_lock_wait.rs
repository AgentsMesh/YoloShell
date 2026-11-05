pub(crate) const RESUME_LIFECYCLE_LOCK_RETRY: Duration =
    crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;
pub(crate) const RESUME_LIFECYCLE_DEADLINE_OVERFLOW: &str =
    "resume lifecycle lock wait deadline overflow";
pub(crate) const RESUME_LIFECYCLE_RETRY_INVALID: &str =
    "resume lifecycle lock retry interval must be positive";

impl Host {
    pub(crate) async fn wait_for_resume_lifecycle_lock(
        &self,
        terminal_id: Uuid,
    ) -> std::result::Result<crate::paths::private_file_lock::PrivateFileLock, ErrorBody> {
        self.wait_for_resume_lifecycle_lock_with_timing(
            terminal_id,
            crate::session_timing::SESSION_RESUME_LIFECYCLE_LOCK_TIMEOUT,
            RESUME_LIFECYCLE_LOCK_RETRY,
        )
        .await
    }

    pub(crate) async fn wait_for_resume_lifecycle_lock_with_timing(
        &self,
        terminal_id: Uuid,
        timeout: Duration,
        retry: Duration,
    ) -> std::result::Result<crate::paths::private_file_lock::PrivateFileLock, ErrorBody> {
        if retry.is_zero() {
            return Err(internal(RESUME_LIFECYCLE_RETRY_INVALID));
        }
        let deadline = tokio::time::Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| internal(RESUME_LIFECYCLE_DEADLINE_OVERFLOW))?;
        loop {
            if tokio::time::Instant::now() >= deadline {
                return Err(ErrorBody::conflict(LIFECYCLE_MUTATION_BUSY));
            }
            if let Some(lock) = self
                .paths
                .try_acquire_session_lifecycle_lock(terminal_id)
                .map_err(internal)?
            {
                return Ok(lock);
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            tokio::time::sleep(remaining.min(retry)).await;
        }
    }

    #[cfg(test)]
    pub(crate) async fn wait_for_resume_lifecycle_lock_test(
        &self,
        terminal_id: Uuid,
        timeout: Duration,
        retry: Duration,
    ) -> std::result::Result<crate::paths::private_file_lock::PrivateFileLock, ErrorBody> {
        self.wait_for_resume_lifecycle_lock_with_timing(terminal_id, timeout, retry)
            .await
    }
}
