use super::*;

const POST_PREPARED_FAIL_STOP_CONTEXT: &str =
    "Host cannot safely continue after ambiguous post-Prepared PTY custody failure";
const POST_PREPARED_CANCEL_DETAIL: &str =
    "Host PTY custody operation was cancelled after the Prepared risk boundary";

pub(super) struct PostPreparedFailStop {
    request_drain: HostRequestDrain,
    cancellation_detail: Option<&'static str>,
    // Drop publishes the sticky failure before releasing the registry gate.
    gate: Option<tokio::sync::OwnedMutexGuard<()>>,
}

impl PostPreparedFailStop {
    pub(super) fn new(request_drain: HostRequestDrain) -> Self {
        Self {
            request_drain,
            cancellation_detail: None,
            gate: None,
        }
    }

    pub(super) fn arm(&mut self) {
        self.cancellation_detail = Some(POST_PREPARED_CANCEL_DETAIL);
    }

    pub(super) fn bind_gate(&mut self, gate: tokio::sync::OwnedMutexGuard<()>) {
        debug_assert!(self.gate.is_none());
        self.gate = Some(gate);
    }

    pub(super) fn record_failure(&self, error: &WorkerRpcError) {
        if self.cancellation_detail.is_some() {
            let detail = super::containment::display_detail(error);
            super::containment::fail_stop(
                &self.request_drain,
                format!("{POST_PREPARED_FAIL_STOP_CONTEXT}: {detail}"),
            );
        }
    }

    pub(super) fn disarm(&mut self) {
        self.cancellation_detail = None;
    }
}

impl Drop for PostPreparedFailStop {
    fn drop(&mut self) {
        if let Some(detail) = self.cancellation_detail {
            super::containment::fail_stop(&self.request_drain, detail.to_owned());
        }
    }
}
