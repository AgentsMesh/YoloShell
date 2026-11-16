use tokio::time::Instant;

const RETAINED_CUSTODY_DETAIL: &str = "PTY custody guardian retained fail-closed custody";

pub(super) struct ProgressFailureLog(super::settlement_backoff::FailureLogBackoff);

impl ProgressFailureLog {
    pub(super) fn report(&mut self, error: anyhow::Error, namespace_error: Option<anyhow::Error>) {
        if !self.0.should_emit(Instant::now()) {
            super::terminal_reporting::discard_owned(error);
            if let Some(namespace_error) = namespace_error {
                super::terminal_reporting::discard_owned(namespace_error);
            }
            return;
        }
        match namespace_error {
            Some(namespace_error) => super::terminal_reporting::error_pair_owned(
                error,
                namespace_error,
                RETAINED_CUSTODY_DETAIL,
            ),
            None => super::terminal_reporting::error_owned(error, RETAINED_CUSTODY_DETAIL),
        }
    }
}

impl Default for ProgressFailureLog {
    fn default() -> Self {
        Self(super::settlement_backoff::FailureLogBackoff::default())
    }
}

pub(super) fn report_throttled(
    log: &mut super::settlement_backoff::FailureLogBackoff,
    error: anyhow::Error,
    message: &'static str,
) {
    if log.should_emit(Instant::now()) {
        super::terminal_reporting::error_owned(error, message);
    } else {
        super::terminal_reporting::discard_owned(error);
    }
}
