use super::constants::*;
use crate::process::ProcessIdentity;
use anyhow::{Context, Result, ensure};
use tokio::time::Instant as TokioInstant;

#[cfg(target_os = "linux")]
#[path = "bootstrap_exec_observer_linux.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "bootstrap_exec_observer_macos.rs"]
mod platform;

#[derive(Clone, Copy)]
pub(crate) struct BootstrapExecTarget {
    pub(crate) process: ProcessIdentity,
    pub(crate) thread_id: u32,
}

pub(crate) struct BootstrapExecObserver(platform::ExecObserver);

impl BootstrapExecObserver {
    pub(crate) fn register(target: BootstrapExecTarget) -> Result<Self> {
        let observer = platform::ExecObserver::register(target)?;
        ensure!(
            target.process.matches(),
            BOOTSTRAP_EXEC_OBSERVER_IDENTITY_CONTEXT
        );
        Ok(Self(observer))
    }

    pub(crate) async fn confirm(self, deadline: TokioInstant) -> Result<()> {
        let observation =
            tokio::task::spawn_blocking(move || self.0.wait_until(deadline.into_std()));
        tokio::time::timeout_at(deadline, observation)
            .await
            .context(BOOTSTRAP_EXEC_OBSERVER_TIMEOUT_CONTEXT)?
            .context(BOOTSTRAP_EXEC_OBSERVER_JOIN_CONTEXT)?
    }
}

pub(crate) fn current_exec_thread_id() -> Result<u32> {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: gettid has no pointer arguments or caller preconditions.
        let thread_id = unsafe { libc::syscall(libc::SYS_gettid) };
        ensure!(
            thread_id > 0 && thread_id <= libc::pid_t::MAX as libc::c_long,
            BOOTSTRAP_EXEC_THREAD_CONTEXT
        );
        Ok(thread_id as u32)
    }
    #[cfg(target_os = "macos")]
    {
        Ok(std::process::id())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        anyhow::bail!(BOOTSTRAP_EXEC_OBSERVER_UNSUPPORTED_CONTEXT)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod platform {
    use super::*;
    use std::time::Instant;

    pub(crate) struct ExecObserver;

    impl ExecObserver {
        pub(crate) fn register(_target: BootstrapExecTarget) -> Result<Self> {
            anyhow::bail!(BOOTSTRAP_EXEC_OBSERVER_UNSUPPORTED_CONTEXT)
        }

        pub(crate) fn wait_until(self, _deadline: Instant) -> Result<()> {
            anyhow::bail!(BOOTSTRAP_EXEC_OBSERVER_UNSUPPORTED_CONTEXT)
        }
    }
}
