impl HostRequestDrain {
    pub(super) async fn wait_for_shutdown(&self) -> Result<(), String> {
        loop {
            let notified = self.0.shutdown.notified();
            if let Some(outcome) = self.shutdown_outcome() {
                return outcome;
            }
            notified.await;
        }
    }

    pub(super) fn fail_stop_detail(&self) -> Option<String> {
        self.is_fail_stopped()
            .then(|| super::lock_recover(&self.0.fail_stop_detail).clone())
            .flatten()
    }

    fn is_fail_stopped(&self) -> bool {
        self.0.fail_stopped.load(Ordering::Acquire)
    }

    fn admission_error(&self) -> Option<ErrorBody> {
        if self.is_fail_stopped() {
            Some(self.fail_stopped_error())
        } else if self.0.draining.load(Ordering::Acquire) {
            Some(draining_error())
        } else {
            None
        }
    }

    fn response_admission_error(&self) -> ErrorBody {
        self.admission_error()
            .unwrap_or_else(|| ErrorBody::conflict(HOST_RESPONSE_ADMISSION_CLOSED))
    }

    fn fail_stopped_error(&self) -> ErrorBody {
        ErrorBody::conflict(self.fail_stop_diagnostic())
    }

    fn fail_stop_diagnostic(&self) -> String {
        super::lock_recover(&self.0.fail_stop_detail)
            .clone()
            .unwrap_or_else(|| HOST_FAIL_STOPPED.to_owned())
    }

    fn shutdown_outcome(&self) -> Option<Result<(), String>> {
        if self.is_fail_stopped() {
            return Some(Err(self.fail_stop_diagnostic()));
        }
        self.0
            .graceful_shutdown
            .load(Ordering::Acquire)
            .then_some(Ok(()))
    }
}
