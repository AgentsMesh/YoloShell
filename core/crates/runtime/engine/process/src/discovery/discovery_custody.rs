use super::discovery_custody_budget::refresh_discovery_observation;
use super::live_identity::ProcessExecutionState;
use super::signal_authority::{ExactProcessSignal, signal_exact_process};
use super::{
    CapturedProcessTree, LiveProcessIdentity, ProcessDiscoveryCapability, ProcessIdentity,
};
use anyhow::{Context, Result, ensure};
use std::time::{Duration, Instant};
const MAX_RECORDED_FAILURES: usize = 4;
const ROOT_MISSING_DETAIL: &str = "suspended custody root is no longer running";
const ROOT_NOT_SUSPENDED_DETAIL: &str = "custody root did not start suspended";
const ROOT_RESUME_DETAIL: &str = "suspended custody root vanished before exact resume";
const SETTLEMENT_TIMEOUT_DETAIL: &str = "discovery process custody did not settle";
const DEADLINE_OVERFLOW_DETAIL: &str = "process custody deadline overflow";
const FAILURE_SEPARATOR: &str = "; ";
pub struct ProcessDiscoveryCustody {
    tree: CapturedProcessTree,
    settled: bool,
}
impl ProcessDiscoveryCustody {
    #[cfg(target_os = "macos")]
    pub fn capture_suspended(
        pid: u32,
        capability: ProcessDiscoveryCapability,
        inheritance: super::ProcessDiscoveryWriterInheritance,
    ) -> Result<Self> {
        let identity = ProcessIdentity::capture(pid)?;
        let root = suspended_root(identity)?;
        capability.commit_writer_inheritance(inheritance, identity)?;
        let mut tree = CapturedProcessTree::capture_with_discovery_internal(
            identity,
            capability.value().to_owned(),
        )?;
        tree.install_process_capability(capability.descriptor_authority())?;
        finish_capture(tree, root)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn capture_suspended(pid: u32, capability: ProcessDiscoveryCapability) -> Result<Self> {
        let identity = ProcessIdentity::capture(pid)?;
        let root = suspended_root(identity)?;
        #[cfg(target_os = "linux")]
        let tree = capability.capture_process_tree(identity)?;
        #[cfg(not(target_os = "linux"))]
        let tree = CapturedProcessTree::capture_with_discovery_internal(
            identity,
            capability.into_value(),
        )?;
        finish_capture(tree, root)
    }

    pub fn refresh(&mut self) -> Result<()> {
        self.tree.refresh().map(|_| ())
    }
    pub fn refresh_if_needed(&mut self) -> Result<()> {
        self.tree.refresh_if_needed_internal().map(|_| ())
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    #[doc(hidden)]
    pub fn fail_next_refresh_for_test(&mut self, detail: &str) {
        self.tree.fail_next_refresh(detail);
    }

    pub fn settle(
        &mut self,
        graceful_timeout: Duration,
        force_timeout: Duration,
        poll_interval: Duration,
    ) -> Result<()> {
        if self.settled {
            return Ok(());
        }
        let mut failures = Vec::new();
        let graceful = deadline(graceful_timeout)?;
        let closed = self.drive(libc::SIGTERM, graceful, poll_interval, &mut failures);
        let closed = if closed {
            true
        } else {
            let force = deadline(force_timeout)?;
            self.drive(libc::SIGKILL, force, poll_interval, &mut failures)
        };
        if closed {
            self.settled = true;
        } else {
            failures.push(SETTLEMENT_TIMEOUT_DETAIL.to_owned());
        }
        ensure!(failures.is_empty(), failures.join(FAILURE_SEPARATOR));
        Ok(())
    }

    fn drive(
        &mut self,
        signal: i32,
        deadline: Instant,
        poll_interval: Duration,
        failures: &mut Vec<String>,
    ) -> bool {
        self.tree.begin_discovery_cleanup_internal();
        while Instant::now() < deadline {
            if let Err(error) = refresh_discovery_observation(&mut self.tree) {
                record(failures, error);
                if let Err(error) = self.signal(signal) {
                    record(failures, error);
                }
                self.tree.begin_discovery_cleanup_internal();
                pause(deadline, poll_interval);
                continue;
            }
            match self.tree.discovery_publication_ready_internal() {
                Ok(true) => return true,
                Ok(false) => {}
                Err(error) => {
                    record(failures, error);
                    if let Err(error) = self.signal(signal) {
                        record(failures, error);
                    }
                    self.tree.begin_discovery_cleanup_internal();
                    pause(deadline, poll_interval);
                    continue;
                }
            }
            let discovery_settled = self.tree.discovery_is_settled_internal();
            match self.tree.live_identities() {
                Ok(identities) if identities.is_empty() => {}
                Ok(_) => {
                    if let Err(error) = self.signal(signal) {
                        record(failures, error);
                    }
                }
                Err(error) => {
                    record(failures, error);
                    if let Err(error) = self.signal(signal) {
                        record(failures, error);
                    }
                }
            }
            if discovery_settled {
                self.tree.begin_discovery_cleanup_internal();
            }
            pause(deadline, poll_interval);
        }
        false
    }

    fn signal(&mut self, signal: i32) -> Result<Vec<ProcessIdentity>> {
        let result = if signal == libc::SIGTERM {
            self.tree.request_graceful_termination()
        } else {
            self.tree.request_force_termination()
        };
        self.tree.begin_discovery_cleanup_internal();
        result
    }
}

fn suspended_root(identity: ProcessIdentity) -> Result<LiveProcessIdentity> {
    let root = LiveProcessIdentity::capture(identity)?.context(ROOT_MISSING_DETAIL)?;
    ensure!(
        root.execution_state()? == ProcessExecutionState::Stopped,
        ROOT_NOT_SUSPENDED_DETAIL
    );
    Ok(root)
}

fn finish_capture(
    tree: CapturedProcessTree,
    root: LiveProcessIdentity,
) -> Result<ProcessDiscoveryCustody> {
    tree.require_destructive_session()?;
    ensure!(
        signal_exact_process(&root, ExactProcessSignal::ResumeSuspended)?,
        ROOT_RESUME_DETAIL
    );
    Ok(ProcessDiscoveryCustody {
        tree,
        settled: false,
    })
}

fn deadline(duration: Duration) -> Result<Instant> {
    Instant::now()
        .checked_add(duration)
        .context(DEADLINE_OVERFLOW_DETAIL)
}

fn pause(deadline: Instant, interval: Duration) {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if !remaining.is_zero() {
        std::thread::sleep(interval.min(remaining));
    }
}

fn record(failures: &mut Vec<String>, error: anyhow::Error) {
    let detail = format!("{error:#}");
    if failures.len() < MAX_RECORDED_FAILURES && !failures.contains(&detail) {
        failures.push(detail);
    }
}
