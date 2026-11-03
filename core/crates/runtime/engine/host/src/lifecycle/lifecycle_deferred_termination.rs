pub(crate) const DEFERRED_EXIT_SETTLEMENT_TIMEOUT_LOG: &str =
    "deferred PTY termination did not publish exact durable exit evidence";

impl Host {
    pub(crate) async fn settle_deferred_termination(
        &self,
        record: &RuntimeRecord,
        error: ErrorBody,
    ) -> std::result::Result<(), ErrorBody> {
        self.settle_deferred_termination_with_timeout(
            record,
            error,
            DEFERRED_TERMINATION_SETTLEMENT_TIMEOUT,
        )
        .await
    }

    pub(crate) async fn settle_deferred_termination_with_timeout(
        &self,
        record: &RuntimeRecord,
        error: ErrorBody,
        timeout: Duration,
    ) -> std::result::Result<(), ErrorBody> {
        if error.code != crate::protocol::error_code::CLEANUP_DEFERRED {
            return Err(error);
        }
        let settled = self.deferred_exit_is_durable(record, timeout).await;
        if settled { Ok(()) } else { Err(error) }
    }

    pub(crate) async fn deferred_exit_is_durable(
        &self,
        record: &RuntimeRecord,
        timeout: Duration,
    ) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if self.exact_deferred_exit_is_durable(record) {
                return true;
            }
            let now = tokio::time::Instant::now();
            if now >= deadline {
                self.report_deferred_exit_timeout(record);
                return false;
            }
            tokio::time::sleep_until(std::cmp::min(now + PROCESS_EXIT_POLL_INTERVAL, deadline))
                .await;
        }
    }

    pub(crate) fn current_child_exit_is_durable(&self, record: &RuntimeRecord) -> bool {
        if !process_identity_is_gone(record.child_pid, record.child_start_time) {
            return false;
        }
        let current = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(record.terminal_id));
        let exit =
            self.read_namespace_json::<ExitRecord>(&self.paths.exit_record(record.terminal_id));
        matches!(current, Ok(current) if current.has_same_process_identity(record))
            && matches!(exit, Ok(exit) if exit.runtime_epoch == record.runtime_epoch)
    }

    pub(crate) fn exact_deferred_exit_is_durable(&self, record: &RuntimeRecord) -> bool {
        self.current_child_exit_is_durable(record)
    }

    pub(crate) fn report_deferred_exit_timeout(&self, record: &RuntimeRecord) {
        let child = process::observe_process_identity(record.child_pid, record.child_start_time);
        let current = self
            .read_namespace_json::<RuntimeRecord>(&self.paths.runtime_record(record.terminal_id));
        let exit =
            self.read_namespace_json::<ExitRecord>(&self.paths.exit_record(record.terminal_id));
        let current_matches =
            matches!(current, Ok(current) if current.has_same_process_identity(record));
        let exit_matches = matches!(exit, Ok(exit) if exit.runtime_epoch == record.runtime_epoch);
        tracing::warn!(
            terminal_id = %record.terminal_id,
            runtime_epoch = %record.runtime_epoch,
            ?child,
            current_matches,
            exit_matches,
            DEFERRED_EXIT_SETTLEMENT_TIMEOUT_LOG
        );
    }
}
