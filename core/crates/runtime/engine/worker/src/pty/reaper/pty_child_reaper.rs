use super::pty_ownership::ReapOutcome;
pub(crate) const DIRECT_CHILD_WAIT_CONTEXT: &str = "wait for direct PTY child";

pub(crate) fn wait_direct_child(process_id: u32) -> ReapOutcome {
    match crate::process::wait_direct_child(process_id) {
        Ok(status) => ReapOutcome::waited(Some(status)),
        Err(error) => failed(error).with_context(DIRECT_CHILD_WAIT_CONTEXT),
    }
}

pub(crate) trait ReapOutcomeContext {
    fn with_context(self, context: &'static str) -> Self;
}

impl ReapOutcomeContext for ReapOutcome {
    fn with_context(mut self, context: &'static str) -> Self {
        self.fault = self.fault.take().map(|error| error.context(context));
        self
    }
}

pub(crate) fn failed(error: impl Into<anyhow::Error>) -> ReapOutcome {
    let mut outcome = ReapOutcome::lost_unknown();
    outcome.fault = Some(error.into());
    outcome
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/ownership/pty_ownership_tests.rs"]
mod tests;
