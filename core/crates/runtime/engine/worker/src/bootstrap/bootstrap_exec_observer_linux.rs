use super::*;
use anyhow::{Context, anyhow, bail, ensure};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Instant;

pub(crate) const INVALID_SYSTEM_RESULT: libc::c_long = -1;
pub(crate) const NO_SIGNAL: libc::c_int = 0;
pub(crate) const PTRACE_EVENT_STATUS_SHIFT: u32 = 16;
pub(crate) const WAIT_OPTIONS: libc::c_int = libc::__WALL | libc::__WNOTHREAD | libc::WNOHANG;

pub(crate) struct ExecObserver {
    pub(crate) cancel: Arc<AtomicBool>,
    pub(crate) result: mpsc::Receiver<Result<()>>,
    pub(crate) handle: Option<JoinHandle<()>>,
}

impl ExecObserver {
    pub(crate) fn register(target: BootstrapExecTarget) -> Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let thread_cancel = Arc::clone(&cancel);
        let (ready_tx, ready_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let handle = std::thread::Builder::new()
            .name(BOOTSTRAP_EXEC_TRACE_THREAD_NAME.to_owned())
            .spawn(move || trace(target, thread_cancel, ready_tx, result_tx))
            .context(BOOTSTRAP_EXEC_TRACE_SPAWN_CONTEXT)?;
        let mut observer = Self {
            cancel,
            result: result_rx,
            handle: Some(handle),
        };
        match ready_rx.recv_timeout(BOOTSTRAP_GATE_TIMEOUT) {
            Ok(Ok(())) => Ok(observer),
            Ok(Err(error)) => {
                observer.cancel_and_join();
                Err(error)
            }
            Err(error) => {
                observer.cancel_and_join();
                Err(error).context(BOOTSTRAP_EXEC_TRACE_READY_CONTEXT)
            }
        }
    }

    pub(crate) fn wait_until(mut self, deadline: Instant) -> Result<()> {
        let timeout = deadline.saturating_duration_since(Instant::now());
        let outcome = self
            .result
            .recv_timeout(timeout)
            .context(BOOTSTRAP_EXEC_OBSERVER_TIMEOUT_CONTEXT)?;
        self.join()?;
        outcome
    }

    pub(crate) fn cancel_and_join(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Err(error) = self.join() {
            tracing::error!(%error, BOOTSTRAP_EXEC_TRACE_STOP_LOG);
        }
    }

    pub(crate) fn join(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| anyhow!(BOOTSTRAP_EXEC_TRACE_JOIN_CONTEXT))?;
        }
        Ok(())
    }
}

impl Drop for ExecObserver {
    fn drop(&mut self) {
        self.cancel_and_join();
    }
}

#[path = "bootstrap_exec_observer_linux_trace.rs"]
mod trace_impl;
use trace_impl::trace;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-bootstrap"))]
#[path = "../../unit_tests/bootstrap/bootstrap_exec_observer_linux_tests.rs"]
mod tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/bootstrap/bootstrap_exec_observer_linux_thread_tests.rs"]
mod thread_tests;
