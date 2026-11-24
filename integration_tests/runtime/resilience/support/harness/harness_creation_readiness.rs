use super::*;
use anyhow::{Context, Result, bail, ensure};
use std::future::Future;
use tokio::time::{Instant, sleep_until, timeout_at};
use yolo_runtime::TerminalView;

pub(super) const CREATED_RUNTIME_NOT_READY: &str =
    "created runtime did not provide an authenticated live view";
const CREATED_RUNTIME_MISSING: &str = "confirmed creation has no runtime view";
const CREATED_RUNTIME_STOPPED: &str = "created runtime stopped before fixture custody";

impl Harness {
    pub(super) async fn wait_for_created_view(&self, terminal_id: Uuid) -> TerminalView {
        let record: RuntimeRecord = read_json(&self.paths().runtime_record(terminal_id)).unwrap();
        let client = self.client();
        wait_for_created_view(&record, Instant::now() + HOST_READY_TIMEOUT, || {
            client.get_session(terminal_id)
        })
        .await
        .unwrap_or_else(|error| panic!("{error:#}; host_stderr={}", self.host_stderr_tail()))
    }
}

/// Creation is issued exactly once. A transient unauthenticated Get cannot grant
/// cleanup authority, and subsequent reads must describe the same created runtime.
pub(super) async fn wait_for_created_view<F, Fut>(
    expected: &RuntimeRecord,
    deadline: Instant,
    mut inspect: F,
) -> Result<TerminalView>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = std::result::Result<TerminalView, ClientError>>,
{
    loop {
        ensure!(Instant::now() < deadline, CREATED_RUNTIME_NOT_READY);
        let view = timeout_at(deadline, inspect())
            .await
            .context(CREATED_RUNTIME_NOT_READY)??;
        ensure!(Instant::now() < deadline, CREATED_RUNTIME_NOT_READY);
        let runtime = view.runtime.as_ref().context(CREATED_RUNTIME_MISSING)?;
        runtime_record_authority::ensure_view_matches(
            expected,
            view.manifest.terminal_id,
            runtime,
        )?;
        match runtime.status {
            TerminalRuntimeStatus::Unreachable => {}
            TerminalRuntimeStatus::Exited | TerminalRuntimeStatus::Lost => {
                bail!("{CREATED_RUNTIME_STOPPED}: {:?}", runtime.status)
            }
            TerminalRuntimeStatus::Starting
            | TerminalRuntimeStatus::Running
            | TerminalRuntimeStatus::Finalizing
            | TerminalRuntimeStatus::OutputBlocked => return Ok(view),
        }
        sleep_until(std::cmp::min(
            Instant::now() + HOST_READY_POLL_INTERVAL,
            deadline,
        ))
        .await;
    }
}
