use super::constants::{
    BOOTSTRAP_EXEC_FAILURE_CONTEXT, BOOTSTRAP_GATE_TIMEOUT_CONTEXT,
    BOOTSTRAP_TEST_PRE_EXEC_MARKER_CONTENT, BOOTSTRAP_TEST_PRE_EXEC_MARKER_ENV,
    BOOTSTRAP_TEST_PRE_EXEC_MARKER_WRITE_CONTEXT,
};
use crate::local_unix::LocalUnixStream;
use anyhow::{Context, Result};
use tokio::io::AsyncWriteExt;

pub(in crate::worker) const EXEC_REPORT_MODE_ENV: &str = "TEST_BOOTSTRAP_EXEC_REPORT_MODE";
pub(in crate::worker) const EXEC_REPORT_MODE_MALFORMED: &str = "malformed";
pub(in crate::worker) const EXEC_REPORT_MODE_TIMEOUT: &str = "timeout";
pub(in crate::worker) const EXEC_REPORT_MARKER_ENV: &str = "TEST_BOOTSTRAP_EXEC_REPORT_MARKER";
pub(super) const EXEC_REPORT_STOP_DETAIL: &str = "injected uncertain bootstrap exec report";
const PARTIAL_FRAME_HEADER: &[u8] = &[u8::MIN];
const EXEC_REPORT_MARKER_CONTEXT: &str = "publish injected bootstrap exec report marker";

pub(in crate::worker) fn expected_failure_context(mode: &str) -> Option<&'static str> {
    match mode {
        EXEC_REPORT_MODE_MALFORMED => Some(BOOTSTRAP_EXEC_FAILURE_CONTEXT),
        EXEC_REPORT_MODE_TIMEOUT => Some(BOOTSTRAP_GATE_TIMEOUT_CONTEXT),
        _ => None,
    }
}

pub(super) async fn inject_exec_report(stream: &mut LocalUnixStream) -> Result<bool> {
    match std::env::var(EXEC_REPORT_MODE_ENV).as_deref() {
        Ok(mode @ EXEC_REPORT_MODE_MALFORMED) => {
            publish_exec_report_marker(mode)?;
            stream.write_all(PARTIAL_FRAME_HEADER).await?;
            stream.shutdown().await?;
            Ok(true)
        }
        Ok(mode @ EXEC_REPORT_MODE_TIMEOUT) => {
            publish_exec_report_marker(mode)?;
            std::future::pending().await
        }
        _ => Ok(false),
    }
}

fn publish_exec_report_marker(mode: &str) -> Result<()> {
    let marker = std::env::var_os(EXEC_REPORT_MARKER_ENV).context(EXEC_REPORT_MARKER_CONTEXT)?;
    crate::paths::atomic_write_json(std::path::Path::new(&marker), &mode)
        .context(EXEC_REPORT_MARKER_CONTEXT)
}

pub(super) fn pause_before_exec() -> Result<()> {
    let Some(marker) = std::env::var_os(BOOTSTRAP_TEST_PRE_EXEC_MARKER_ENV) else {
        return Ok(());
    };
    std::fs::write(marker, BOOTSTRAP_TEST_PRE_EXEC_MARKER_CONTENT)
        .context(BOOTSTRAP_TEST_PRE_EXEC_MARKER_WRITE_CONTEXT)?;
    loop {
        std::thread::park();
    }
}
