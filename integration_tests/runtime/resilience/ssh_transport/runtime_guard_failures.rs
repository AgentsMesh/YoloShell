use anyhow::{Result, anyhow};

const DETACHED_CLEANUP_FAILURE_DETAIL: &str = "detached runtime cleanup failed";
const FAILURE_SEPARATOR: &str = "\n";
const FAILURE_DISPLAY_OFFSET: usize = 1;

pub(super) fn remember_failure(failures: &mut Vec<anyhow::Error>, result: Result<()>) {
    if let Err(error) = result {
        failures.push(error);
    }
}

pub(super) fn finish_failures(failures: Vec<anyhow::Error>) -> Result<()> {
    if failures.is_empty() {
        return Ok(());
    }
    let details = failures
        .into_iter()
        .enumerate()
        .map(|(index, error)| format!("{}. {error:#}", index + FAILURE_DISPLAY_OFFSET))
        .collect::<Vec<_>>()
        .join(FAILURE_SEPARATOR);
    Err(anyhow!("{DETACHED_CLEANUP_FAILURE_DETAIL}:\n{details}"))
}

pub(super) fn report_drop_failures(failures: Vec<anyhow::Error>) {
    let Err(error) = finish_failures(failures) else {
        return;
    };
    if std::thread::panicking() {
        tracing::error!(%error, "{DETACHED_CLEANUP_FAILURE_DETAIL}");
    } else {
        panic!("{error:#}");
    }
}
