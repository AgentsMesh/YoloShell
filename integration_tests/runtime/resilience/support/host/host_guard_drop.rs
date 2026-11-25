use super::stop::report_stop_failure;
use super::*;
use anyhow::{Context, Result};
use std::os::unix::process::ExitStatusExt;

const OWNED_HOST_KILL_CONTEXT: &str = "force owned test Host child";
const OWNED_HOST_WAIT_CONTEXT: &str = "reap owned test Host child";
const OWNED_HOST_FALLBACK_CONTEXT: &str = "owned Host fallback cleanup failed";

impl Drop for HostGuard {
    fn drop(&mut self) {
        let Err(mut error) = self.stop_result() else {
            return;
        };
        if let Err(fallback) = force_owned_child(self) {
            error = error.context(format!("{OWNED_HOST_FALLBACK_CONTEXT}: {fallback:#}"));
        }
        report_stop_failure(error);
    }
}

fn force_owned_child(host: &mut HostGuard) -> Result<()> {
    let Some(mut child) = host.child.take() else {
        return Ok(());
    };
    if let Err(error) = child.kill().context(OWNED_HOST_KILL_CONTEXT) {
        match child.try_wait().context(OWNED_HOST_WAIT_CONTEXT) {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => {}
            Err(wait_error) => {
                host.child = Some(child);
                return Err(wait_error);
            }
        }
        host.child = Some(child);
        return Err(error);
    }
    let status = child.wait().context(OWNED_HOST_WAIT_CONTEXT)?;
    if status.signal() == Some(libc::SIGKILL) {
        discard_incomplete_coverage_profile(child.id());
    }
    Ok(())
}
