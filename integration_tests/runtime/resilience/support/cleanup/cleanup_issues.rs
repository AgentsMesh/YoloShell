use anyhow::{Result, anyhow};

const CLEANUP_ISSUES_DETAIL: &str = "runtime cleanup reported multiple failures";
const DISPLAY_INDEX_OFFSET: usize = 1;
const FAILURE_SEPARATOR: &str = "\n";

#[derive(Default)]
pub(super) struct CleanupIssues {
    failures: Vec<anyhow::Error>,
}

impl CleanupIssues {
    pub(super) fn record<T>(&mut self, result: Result<T>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.failures.push(error);
                None
            }
        }
    }

    pub(super) fn record_error(&mut self, error: Option<anyhow::Error>) {
        if let Some(error) = error {
            self.failures.push(error);
        }
    }

    pub(super) fn record_errors(&mut self, errors: impl IntoIterator<Item = anyhow::Error>) {
        self.failures.extend(errors);
    }

    pub(super) fn merge(&mut self, mut other: Self) {
        self.failures.append(&mut other.failures);
    }

    pub(super) fn check(&self) -> Result<()> {
        if self.failures.is_empty() {
            return Ok(());
        }
        let details = self
            .failures
            .iter()
            .enumerate()
            .map(|(index, error)| format!("{}. {error:#}", index + DISPLAY_INDEX_OFFSET))
            .collect::<Vec<_>>()
            .join(FAILURE_SEPARATOR);
        Err(anyhow!("{CLEANUP_ISSUES_DETAIL}:\n{details}"))
    }

    pub(super) fn finish(self) -> Result<()> {
        self.check()
    }
}
