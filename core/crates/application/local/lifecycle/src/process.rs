use crate::constants::*;
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::path::Path;
use tokio::time::{Instant, sleep};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ServiceStop {
    pub service: &'static str,
    pub pid: Option<u32>,
    pub stopped: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StopReport {
    pub services: Vec<ServiceStop>,
}

/// Resolves the process that owns a private service socket. The peer must be
/// the current user; no process-name matching is involved.
pub async fn service_pid(path: &Path) -> Option<u32> {
    let stream = tokio::net::UnixStream::connect(path).await.ok()?;
    let credentials = stream.peer_cred().ok()?;
    // SAFETY: geteuid has no preconditions and only reads process credentials.
    if credentials.uid() != unsafe { libc::geteuid() } {
        return None;
    }
    credentials.pid().and_then(|pid| u32::try_from(pid).ok())
}

pub(crate) async fn stop_socket(
    path: &Path,
    service: &'static str,
    deadline: Instant,
) -> Result<ServiceStop> {
    let Some(pid) = service_pid(path).await else {
        return Ok(ServiceStop {
            service,
            pid: None,
            stopped: false,
        });
    };
    signal_term(pid)?;
    ensure!(wait_exit(pid, deadline).await, STOP_TIMED_OUT);
    Ok(ServiceStop {
        service,
        pid: Some(pid),
        stopped: true,
    })
}

fn signal_term(pid: u32) -> Result<()> {
    let result = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
    if result == 0 {
        return Ok(());
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(());
    }
    Err(error).with_context(|| SIGNAL_FAILED)
}

async fn wait_exit(pid: u32, deadline: Instant) -> bool {
    loop {
        if !is_alive(pid) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        sleep(STOP_POLL).await;
    }
}

fn is_alive(pid: u32) -> bool {
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if result == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
}
