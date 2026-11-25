use super::constants::*;
use anyhow::{Context, Result, ensure};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::process::ChildStderr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Instant;

pub(super) struct WatchdogDiagnostics {
    reader: Option<JoinHandle<std::io::Result<CapturedDiagnostic>>>,
    cancelled: Arc<AtomicBool>,
    collected: Option<std::result::Result<String, String>>,
}

struct CapturedDiagnostic {
    bytes: Vec<u8>,
    truncated: bool,
}

impl WatchdogDiagnostics {
    pub(super) fn start(stderr: ChildStderr) -> Result<Self> {
        enable_nonblocking(&stderr)?;
        let cancelled = Arc::new(AtomicBool::new(false));
        let reader_cancelled = Arc::clone(&cancelled);
        let reader = std::thread::Builder::new()
            .name(WATCHDOG_DIAGNOSTIC_THREAD_NAME.to_owned())
            .spawn(move || read_bounded_until(stderr, || reader_cancelled.load(Ordering::Acquire)))
            .context(WATCHDOG_DIAGNOSTIC_START_DETAIL)?;
        Ok(Self {
            reader: Some(reader),
            cancelled,
            collected: None,
        })
    }

    pub(super) fn collect(&mut self) -> Result<String> {
        if self.collected.is_none() {
            self.collected = Some(self.join().map_err(|error| format!("{error:#}")));
        }
        match self.collected.as_ref().unwrap() {
            Ok(value) => Ok(value.clone()),
            Err(detail) => Err(anyhow::anyhow!(detail.clone())),
        }
    }

    fn join(&mut self) -> Result<String> {
        if !self.await_reader(Instant::now() + WATCHDOG_DIAGNOSTIC_DRAIN_TIMEOUT) {
            self.cancelled.store(true, Ordering::Release);
            ensure!(
                self.await_reader(Instant::now() + WATCHDOG_DIAGNOSTIC_CANCEL_TIMEOUT),
                WATCHDOG_DIAGNOSTIC_CANCEL_DETAIL
            );
        }
        let reader = self
            .reader
            .take()
            .context(WATCHDOG_DIAGNOSTIC_READ_DETAIL)?;
        let captured = reader
            .join()
            .map_err(|_| anyhow::anyhow!(WATCHDOG_DIAGNOSTIC_THREAD_DETAIL))?
            .context(WATCHDOG_DIAGNOSTIC_READ_DETAIL)?;
        Ok(render(captured))
    }

    fn await_reader(&self, deadline: Instant) -> bool {
        loop {
            if self
                .reader
                .as_ref()
                .is_none_or(std::thread::JoinHandle::is_finished)
            {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(PROCESS_POLL_INTERVAL);
        }
    }
}

fn read_bounded(mut reader: impl Read) -> std::io::Result<CapturedDiagnostic> {
    read_bounded_until(&mut reader, || false)
}

fn read_bounded_until(
    mut reader: impl Read,
    cancelled: impl Fn() -> bool,
) -> std::io::Result<CapturedDiagnostic> {
    let mut bytes = Vec::with_capacity(MAXIMUM_WATCHDOG_DIAGNOSTIC_BYTES);
    let mut truncated = false;
    let mut buffer = [u8::default(); CONTROL_READ_BUFFER_BYTES];
    loop {
        if cancelled() {
            return Ok(CapturedDiagnostic { bytes, truncated });
        }
        let read = match reader.read(&mut buffer) {
            Ok(read) => read,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(PROCESS_POLL_INTERVAL);
                continue;
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if read == usize::default() {
            return Ok(CapturedDiagnostic { bytes, truncated });
        }
        let available = MAXIMUM_WATCHDOG_DIAGNOSTIC_BYTES.saturating_sub(bytes.len());
        let retained = read.min(available);
        bytes.extend_from_slice(&buffer[..retained]);
        truncated |= retained < read;
    }
}

fn enable_nonblocking(stderr: &ChildStderr) -> Result<()> {
    let descriptor = stderr.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    ensure!(
        flags != INVALID_SYSTEM_RESULT,
        std::io::Error::last_os_error()
    );
    let result = unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    ensure!(
        result != INVALID_SYSTEM_RESULT,
        std::io::Error::last_os_error()
    );
    Ok(())
}

fn render(captured: CapturedDiagnostic) -> String {
    let mut detail = String::from_utf8_lossy(&captured.bytes)
        .trim_end_matches(['\r', '\n'])
        .to_owned();
    if captured.truncated {
        detail.push_str(WATCHDOG_OUTPUT_TRUNCATED_SUFFIX);
    }
    detail
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))] #[path = "../unit_tests/watchdog_diagnostics_tests.rs"]
mod tests;
