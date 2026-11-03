use super::MacosProcessCapabilityManagerReader;
use crate::process::LiveProcessIdentity;
#[cfg(any(test, feature = "runtime-component-tests"))]
use crate::process::ProcessIdentity;
use anyhow::{Context, Result, ensure};
use std::os::fd::{AsRawFd, OwnedFd};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[path = "macos_process_capability_monitor_kqueue.rs"]
mod kqueue;
#[path = "macos_process_capability_monitor_reconciliation.rs"]
mod reconciliation;
#[path = "macos_process_capability_monitor_state.rs"]
mod state;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/macos/macos_process_capability_monitor_state_tests.rs"]
mod state_tests;

const MONITOR_SPAWN_CONTEXT: &str = "start process capability liveness monitor";
const MONITOR_ROOT_DETAIL: &str = "process capability monitor root identity changed";
const MONITOR_THREAD_NAME: &str = "yolo-process-capability";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacosProcessCapabilityObservation {
    WriterPresent,
    ReleasedPendingRootExit,
    RootExited,
    ReleasedAfterRootExit,
    PrematurelyReleased,
    Indeterminate,
}

pub(super) struct MacosProcessCapabilityMonitor {
    root: LiveProcessIdentity,
    queue: Arc<kqueue::MonitorQueue>,
    shared: Arc<MonitorShared>,
}

struct MonitorShared {
    progress: Mutex<MonitorProgress>,
}

struct MonitorProgress {
    observation: MacosProcessCapabilityObservation,
    release_deadline: Option<Instant>,
}

impl MacosProcessCapabilityManagerReader {
    pub(crate) fn arm_liveness(&self, root: LiveProcessIdentity) -> Result<()> {
        let mut monitor = self
            .monitor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(monitor) = monitor.as_ref() {
            ensure!(monitor.root == root, MONITOR_ROOT_DETAIL);
            return Ok(());
        }
        *monitor = Some(MacosProcessCapabilityMonitor::start(
            Arc::clone(&self.descriptor),
            root,
        )?);
        Ok(())
    }

    pub(crate) fn confirm_root_exit(&self, root: &LiveProcessIdentity) -> Result<()> {
        let monitor = self
            .monitor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(monitor) = monitor.as_ref() else {
            return Ok(());
        };
        monitor.confirm_root_exit(&self.descriptor, root)
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn arm_liveness_for_test(&self, identity: ProcessIdentity) -> Result<()> {
        let live = LiveProcessIdentity::capture(identity)?.context(MONITOR_ROOT_DETAIL)?;
        self.arm_liveness(live)
    }
}

impl MacosProcessCapabilityMonitor {
    fn start(descriptor: Arc<OwnedFd>, root: LiveProcessIdentity) -> Result<Self> {
        ensure!(root.try_matches()?, MONITOR_ROOT_DETAIL);
        let queue = Arc::new(kqueue::MonitorQueue::create(
            descriptor.as_raw_fd(),
            root.pid(),
        )?);
        ensure!(root.try_matches()?, MONITOR_ROOT_DETAIL);
        let shared = Arc::new(MonitorShared {
            progress: Mutex::new(MonitorProgress {
                observation: MacosProcessCapabilityObservation::WriterPresent,
                release_deadline: None,
            }),
        });
        let watched_queue = Arc::clone(&queue);
        let watched_shared = Arc::clone(&shared);
        let watched_root = root.clone();
        std::thread::Builder::new()
            .name(MONITOR_THREAD_NAME.to_owned())
            .spawn(move || watch(descriptor, watched_root, watched_queue, watched_shared))
            .context(MONITOR_SPAWN_CONTEXT)?;
        Ok(Self {
            root,
            queue,
            shared,
        })
    }

    pub(super) fn observation(&self) -> MacosProcessCapabilityObservation {
        self.shared
            .progress
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .observation
    }
}

fn watch(
    descriptor: Arc<OwnedFd>,
    root: LiveProcessIdentity,
    queue: Arc<kqueue::MonitorQueue>,
    shared: Arc<MonitorShared>,
) {
    loop {
        let deadline = lock_progress(&shared).release_deadline;
        let event = queue.next_event(deadline);
        let mut progress = lock_progress(&shared);
        match &event {
            Ok(kqueue::MonitorEvent::RootExit) => {
                state::observe_kernel_root_exit(&descriptor, &mut progress)
            }
            Ok(kqueue::MonitorEvent::Wake) => {}
            Ok(kqueue::MonitorEvent::Descriptor) => {
                state::observe_descriptor(&descriptor, &root, &mut progress)
            }
            Ok(kqueue::MonitorEvent::Deadline) => {
                state::observe_release_deadline(&root, &mut progress)
            }
            Err(_) => progress.observation = MacosProcessCapabilityObservation::Indeterminate,
        }
        if state::is_terminal(progress.observation) {
            return;
        }
    }
}

fn lock_progress(shared: &MonitorShared) -> std::sync::MutexGuard<'_, MonitorProgress> {
    shared
        .progress
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
