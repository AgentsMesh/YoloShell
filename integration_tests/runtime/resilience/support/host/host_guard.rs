use super::*;
use anyhow::Result;
use std::process::ExitStatus;
use yolo_process_test_support::ProcessIdentityObservation;

const HOST_IDENTITY_ERROR: &str = "cannot establish test host process identity";
const HOST_LOG_IDENTITY_ERROR: &str = "replacement Host must append to the existing stderr log";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ProcessIdentity {
    pub(crate) pid: u32,
    pub(crate) start_time: u64,
}

impl ProcessIdentity {
    pub(crate) fn capture(pid: u32) -> Option<Self> {
        yolo_process_test_support::process_start_time(pid)
            .ok()
            .map(|start_time| Self { pid, start_time })
    }

    pub(crate) fn matches(self) -> bool {
        self.observe() == ProcessIdentityObservation::Matching
    }

    pub(crate) fn observe(self) -> ProcessIdentityObservation {
        yolo_process_test_support::observe_process_identity(self.pid, self.start_time)
    }

    pub(crate) fn signal(self, signal: i32) -> anyhow::Result<bool> {
        yolo_process_test_support::ProcessIdentity {
            pid: self.pid,
            start_time: self.start_time,
        }
        .signal(signal)
    }
}

pub(crate) struct HostGuard {
    child: Option<Child>,
    expected_home: PathBuf,
    identity: ProcessIdentity,
    log: host_log::HostLog,
}

impl HostGuard {
    pub(crate) fn new(mut spawned: SpawnedHost) -> Self {
        let child = &mut spawned.child;
        let Some(identity) = ProcessIdentity::capture(child.id()) else {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{HOST_IDENTITY_ERROR}");
        };
        Self {
            child: Some(spawned.child),
            expected_home: spawned.home,
            identity,
            log: spawned.log,
        }
    }

    pub(crate) fn require_same_log(&self, replacement: &SpawnedHost) -> Result<(), &'static str> {
        (self.log.identity() == replacement.log_identity())
            .then_some(())
            .ok_or(HOST_LOG_IDENTITY_ERROR)
    }

    pub(crate) fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        match self.child.as_mut() {
            Some(child) => child.try_wait(),
            None => Ok(None),
        }
    }

    pub(crate) fn stderr_tail(&self) -> String {
        self.log.tail()
    }

    pub(crate) fn identity(&self) -> ProcessIdentity {
        self.identity
    }

    #[cfg(test)]
    pub(super) fn is_reaped(&self) -> bool {
        self.child.is_none()
    }
}

#[path = "host_guard_drop.rs"]
mod drop_cleanup;
#[path = "host_guard_status.rs"]
mod status_validation;
#[path = "host_guard_stop.rs"]
mod stop;
