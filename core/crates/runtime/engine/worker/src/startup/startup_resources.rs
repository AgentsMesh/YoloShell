#[cfg(test)]
use super::pty_output_progress::ExclusivePtyOutput;
#[cfg(test)]
use super::startup_guard::PtyChild;
use super::startup_guard::StartupChildGuard;
use anyhow::{Context, Result};

pub(crate) const STARTUP_MASTER_UNAVAILABLE_DETAIL: &str = "startup PTY master is unavailable";
pub(crate) const BOOTSTRAP_EXEC_AUTHORITY_UNAVAILABLE_DETAIL: &str =
    "startup bootstrap exec-transition authority is unavailable";
pub(crate) const BOOTSTRAP_CANCELLATION_FAILURE_DETAIL: &str =
    "failed to record bootstrap cancellation before gate release";

pub(crate) struct StartupPtyResources<M> {
    // Rust drops fields in declaration order: hand off output authority before closing the master.
    pub(crate) child: StartupChildGuard,
    pub(crate) master: Option<M>,
    pub(crate) bootstrap_exec: Option<crate::process::BootstrapExecTransitionAuthority>,
}

impl<M> StartupPtyResources<M> {
    #[cfg(test)]
    pub(crate) fn new(master: M, child: PtyChild) -> Self {
        Self {
            child: StartupChildGuard::new(child),
            master: Some(master),
            bootstrap_exec: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_output(master: M, child: PtyChild, output: ExclusivePtyOutput) -> Self {
        Self {
            child: StartupChildGuard::with_output(child, output),
            master: Some(master),
            bootstrap_exec: None,
        }
    }

    pub(crate) fn from_guard(master: M, child: StartupChildGuard) -> Self {
        Self {
            child,
            master: Some(master),
            bootstrap_exec: None,
        }
    }

    pub(crate) fn from_bootstrap_guard(
        master: M,
        child: StartupChildGuard,
        bootstrap_exec: crate::process::BootstrapExecTransitionAuthority,
    ) -> Self {
        Self {
            child,
            master: Some(master),
            bootstrap_exec: Some(bootstrap_exec),
        }
    }

    pub(crate) fn master(&self) -> Result<&M> {
        self.master
            .as_ref()
            .context(STARTUP_MASTER_UNAVAILABLE_DETAIL)
    }

    pub(crate) fn take_master(&mut self) -> Result<M> {
        self.master
            .take()
            .context(STARTUP_MASTER_UNAVAILABLE_DETAIL)
    }

    pub(crate) fn close_owner_master(&mut self) -> Result<()> {
        let master = self
            .master
            .take()
            .context(STARTUP_MASTER_UNAVAILABLE_DETAIL)?;
        drop(master);
        self.child.mark_owner_master_released();
        Ok(())
    }

    pub(crate) fn take_bootstrap_exec(
        &mut self,
    ) -> Result<crate::process::BootstrapExecTransitionAuthority> {
        self.bootstrap_exec
            .take()
            .context(BOOTSTRAP_EXEC_AUTHORITY_UNAVAILABLE_DETAIL)
    }

    pub(crate) fn child(&self) -> &StartupChildGuard {
        &self.child
    }

    pub(crate) fn child_mut(&mut self) -> &mut StartupChildGuard {
        &mut self.child
    }
}

impl<M> Drop for StartupPtyResources<M> {
    fn drop(&mut self) {
        if let Some(authority) = self.bootstrap_exec.take() {
            let cancellation = authority.cancel_before_release().and_then(|proof| {
                self.child
                    .record_explicit_bootstrap_exec_failure_proof(proof)
            });
            if let Err(error) = cancellation {
                super::terminal_reporting::warn_error(error, BOOTSTRAP_CANCELLATION_FAILURE_DETAIL);
            }
        }
        if self.master.is_some() {
            drop(self.master.take());
            self.child.mark_owner_master_released();
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../unit_tests/startup/startup_resources_tests.rs"]
mod tests;
