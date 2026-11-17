use crate::agent::hooks::ApplyDisposition;
use anyhow::Result;

#[must_use]
pub(in crate::worker) struct CodexHookHandleOutcome {
    disposition: Option<ApplyDisposition>,
    response: Result<()>,
}

impl CodexHookHandleOutcome {
    pub(super) fn new(disposition: Option<ApplyDisposition>, response: Result<()>) -> Self {
        Self {
            disposition,
            response,
        }
    }

    pub(in crate::worker) fn into_parts(self) -> (Option<ApplyDisposition>, Result<()>) {
        (self.disposition, self.response)
    }
}
