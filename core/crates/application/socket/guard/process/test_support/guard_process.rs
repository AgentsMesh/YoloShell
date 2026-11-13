use super::constants::*;
use super::process_support;
use crate::fault_contract::{
    DISCOVERY_ARM_ACKNOWLEDGEMENT_ENVIRONMENT, DISCOVERY_ARM_REQUEST_ENVIRONMENT,
    FAILURE_TRIGGER_ENVIRONMENT, FORCE_SIGNAL_ACKNOWLEDGEMENT_ENVIRONMENT,
    FORCE_SIGNAL_REQUEST_ENVIRONMENT, REFRESH_HANDOFF_ACKNOWLEDGEMENT_ENVIRONMENT,
    REFRESH_HANDOFF_REQUEST_ENVIRONMENT,
};
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use std::process::{Child, ExitStatus};
use std::time::Instant;
use tempfile::TempDir;
use yolo_process_test_support::ProcessIdentity;

#[path = "guard_process_refresh.rs"]
mod refresh;
use refresh::{RefreshEvidence, RefreshMilestones};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FailureAction {
    None,
    GuardKill,
    WatchdogKill,
    Exit,
    Panic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LeafState {
    None,
    Running,
    Stopped,
    ForkOnTerm,
    ForkBeforeForce,
    ForkAfterRefresh,
}

pub(super) struct GuardProcess {
    _directory: TempDir,
    pub(super) guard: Option<Child>,
    pub(super) status: Option<ExitStatus>,
    pub(super) guard_identity: ProcessIdentity,
    pub(super) watchdog: Option<ProcessIdentity>,
    pub(super) leaf: Option<ProcessIdentity>,
    pub(super) late_leaf: Option<ProcessIdentity>,
    pub(super) refresh_holder: Option<ProcessIdentity>,
    pub(super) socket_root: Option<PathBuf>,
    ready: PathBuf,
    release: PathBuf,
    failure_trigger: PathBuf,
    leaf_identity_file: PathBuf,
    late_leaf_identity_file: PathBuf,
    refresh_holder_identity_file: PathBuf,
    socket_root_file: PathBuf,
    refresh_postcheck_witness: PathBuf,
    refresh_evidence: Option<RefreshEvidence>,
    leaf_state: LeafState,
    failure: FailureAction,
    triggered: bool,
    pub(super) settled: bool,
}

#[path = "guard_process_spawn.rs"]
mod spawn;

impl GuardProcess {
    pub(super) fn trigger_failure(&mut self) {
        assert_ne!(self.failure, FailureAction::None);
        assert!(!self.triggered);
        match self.failure {
            FailureAction::GuardKill => {
                if self.leaf_state == LeafState::ForkOnTerm {
                    assert!(self.leaf.unwrap().signal(libc::SIGTERM).unwrap());
                }
                process_support::kill_guard(self.guard_identity).unwrap()
            }
            FailureAction::WatchdogKill => {
                assert!(self.watchdog.unwrap().signal(libc::SIGKILL).unwrap())
            }
            FailureAction::Exit | FailureAction::Panic => {
                std::fs::write(&self.failure_trigger, EMPTY_MARKER).unwrap()
            }
            FailureAction::None => unreachable!(),
        }
        self.triggered = true;
    }

    pub(super) fn wait_for_success(&mut self) {
        assert_eq!(self.failure, FailureAction::None);
        std::fs::write(&self.release, EMPTY_MARKER).unwrap();
        self.wait_for_cleanup(true);
    }

    pub(super) fn wait_for_failure_cleanup(&mut self) {
        assert!(self.triggered);
        self.wait_for_cleanup(false);
    }

    pub(super) fn refresh_postcheck_witnessed(&self) -> bool {
        self.refresh_postcheck_witness.exists()
    }

    pub(super) fn refresh_milestones(&self) -> Option<RefreshMilestones> {
        self.refresh_evidence.as_ref().map(RefreshEvidence::observe)
    }

    fn wait_for_ready(&mut self) {
        let deadline = Instant::now() + START_TIMEOUT;
        while !self.ready.exists() {
            if let Some(status) = self.guard.as_mut().unwrap().try_wait().unwrap() {
                self.guard.take();
                self.status = Some(status);
                panic!("{GUARD_EARLY_EXIT_DETAIL}: {status}");
            }
            assert!(Instant::now() < deadline, "{FIXTURE_TIMEOUT_DETAIL}");
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    fn wait_for_cleanup(&mut self, success: bool) {
        let deadline = Instant::now() + CLEANUP_TIMEOUT;
        loop {
            self.refresh_evidence();
            let status = self.reap_guard();
            let identities_gone = !self.guard_identity.matches()
                && self.leaf.is_none_or(|value| !value.matches())
                && self.late_leaf.is_none_or(|value| !value.matches())
                && self.refresh_holder.is_none_or(|value| !value.matches())
                && self.watchdog.is_none_or(|value| !value.matches());
            let root_gone = self.socket_root.as_ref().is_none_or(|path| !path.exists());
            if let Some(status) = status.filter(|_| identities_gone && root_gone) {
                assert_eq!(status.success(), success, "{GUARD_STATUS_DETAIL}: {status}");
                self.settled = true;
                return;
            }
            assert!(
                Instant::now() < deadline,
                "{CLEANUP_TIMEOUT_DETAIL}: {:?}",
                self.refresh_evidence.as_ref().map(RefreshEvidence::observe)
            );
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    pub(super) fn reap_guard(&mut self) -> Option<ExitStatus> {
        if self.status.is_none()
            && let Some(status) = self.guard.as_mut()?.try_wait().unwrap()
        {
            self.guard.take();
            self.status = Some(status);
        }
        self.status
    }

    pub(super) fn refresh_evidence(&mut self) {
        if self.socket_root.is_none()
            && let Ok(value) = std::fs::read(&self.socket_root_file)
        {
            self.socket_root = Some(PathBuf::from(OsString::from_vec(value)));
        }
        if self.leaf.is_none() {
            self.leaf = process_support::try_capture_identity(&self.leaf_identity_file);
        }
        if self.late_leaf.is_none() {
            self.late_leaf = super::process_identity_record::read(&self.late_leaf_identity_file);
        }
        if self.refresh_holder.is_none() {
            self.refresh_holder =
                super::process_identity_record::read(&self.refresh_holder_identity_file);
        }
        if self.watchdog.is_none() {
            self.watchdog = process_support::try_capture_watchdog(self.guard_identity, self.leaf);
        }
    }
}

fn stop_leaf(identity: ProcessIdentity) {
    assert!(identity.signal(libc::SIGSTOP).unwrap());
    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        assert!(identity.matches(), "{STOPPED_LEAF_DETAIL}");
        if identity.is_stably_stopped().unwrap() {
            return;
        }
        assert!(Instant::now() < deadline, "{STOPPED_LEAF_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }
}
