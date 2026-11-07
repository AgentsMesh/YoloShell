use super::super::constants::*;
use super::super::direct_child::force_unreaped_direct_child;
use super::super::protocol::{CompletionFrame, parse_completion, write_capability};
use super::WatchdogClient;
use anyhow::{Context, Result, ensure};
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;
use std::time::Instant;

impl WatchdogClient {
    pub(in crate::socket_root_guard) fn exited_early(&mut self) -> Result<Option<ExitStatus>> {
        self.child
            .as_mut()
            .context(WATCHDOG_EARLY_EXIT_DETAIL)?
            .try_wait()
            .map_err(Into::into)
    }

    pub(in crate::socket_root_guard) fn describe_exit_status(status: ExitStatus) -> String {
        if let Some(signal) = status.signal() {
            format!("{WATCHDOG_EXIT_SIGNAL_DETAIL}: {signal}")
        } else {
            format!("{WATCHDOG_EXIT_STATUS_DETAIL}: {status}")
        }
    }

    pub(in crate::socket_root_guard) fn finish(&mut self) -> Result<()> {
        let mut failures = Vec::new();
        record(&mut failures, self.send_finish());
        let completion = self
            .output
            .read_frame(WATCHDOG_COMPLETION_TIMEOUT)
            .and_then(|value| parse_completion(&value, &self.capability))
            .context(WATCHDOG_COMPLETION_DETAIL);
        record_completion(&mut failures, completion);
        let reaped = record_status(
            &mut failures,
            self.reap_until(Instant::now() + WATCHDOG_FORCE_REAP_TIMEOUT),
        );
        if reaped {
            record_diagnostics(&mut failures, self.diagnostics.collect());
        }
        ensure!(failures.is_empty(), failures.join(FAILURE_SEPARATOR));
        Ok(())
    }

    fn send_finish(&mut self) -> Result<()> {
        let result = (|| {
            write_capability(
                self.input.as_mut().context(WATCHDOG_PIPE_DETAIL)?,
                FRAME_FINISH,
                &self.capability,
            )
        })()
        .context(WATCHDOG_PIPE_DETAIL);
        self.input.take();
        result
    }

    fn reap_until(&mut self, deadline: Instant) -> Result<Option<ExitStatus>> {
        loop {
            if let Some(status) = self
                .child
                .as_mut()
                .context(WATCHDOG_REAP_DETAIL)?
                .try_wait()?
            {
                self.child.take();
                return Ok(Some(status));
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            std::thread::sleep(PROCESS_POLL_INTERVAL);
        }
    }

    fn settle(&mut self) {
        self.input.take();
        if self.child.is_some()
            && self
                .reap_until(Instant::now() + WATCHDOG_DROP_TIMEOUT)
                .ok()
                .flatten()
                .is_none()
        {
            let _ = force_unreaped_direct_child(
                self.child.as_mut().unwrap(),
                Some(self.identity),
                WATCHDOG_REAP_DETAIL,
            );
            let _ = self.reap_until(Instant::now() + WATCHDOG_FORCE_REAP_TIMEOUT);
        }
        if self.child.is_none() {
            let _ = self.diagnostics.collect();
        }
    }
}

impl Drop for WatchdogClient {
    fn drop(&mut self) {
        self.settle();
    }
}

fn record(failures: &mut Vec<String>, result: Result<()>) {
    if let Err(error) = result {
        failures.push(format!("{error:#}"));
    }
}

fn record_completion(failures: &mut Vec<String>, completion: Result<CompletionFrame>) {
    match completion {
        Ok(frame) if frame.completed => {}
        Ok(frame) => failures.push(format!(
            "{WATCHDOG_EXIT_DETAIL}: {}",
            frame
                .detail
                .unwrap_or_else(|| WATCHDOG_PROTOCOL_DETAIL.to_owned())
        )),
        Err(error) => failures.push(format!("{error:#}")),
    }
}

fn record_status(failures: &mut Vec<String>, status: Result<Option<ExitStatus>>) -> bool {
    match status {
        Ok(Some(status)) => {
            if !status.success() {
                failures.push(WatchdogClient::describe_exit_status(status));
            }
            true
        }
        Ok(None) => {
            failures.push(WATCHDOG_REAP_DETAIL.to_owned());
            false
        }
        Err(error) => {
            failures.push(format!("{WATCHDOG_REAP_DETAIL}: {error:#}"));
            false
        }
    }
}

fn record_diagnostics(failures: &mut Vec<String>, diagnostics: Result<String>) {
    match diagnostics {
        Ok(detail) if !detail.is_empty() && !failures.is_empty() => {
            failures.push(format!("{WATCHDOG_DIAGNOSTIC_DETAIL}: {detail}"));
        }
        Ok(_) => {}
        Err(error) => failures.push(format!("{error:#}")),
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/watchdog_client_finish_tests.rs"]
mod tests;
