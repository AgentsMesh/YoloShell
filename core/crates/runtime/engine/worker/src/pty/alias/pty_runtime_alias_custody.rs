use super::SystemMutationPermit;
use super::pty_runtime_alias_closure_task::{DurableAliasClosure, PreparedAliasClosure};
use super::secondary_custody_authority::ReadyWorkerCustodyIdentity;
use super::secondary_custody_release::WorkerCustodyReleaseHandle;
use super::{ControllerMutationPermit, WorkerState, lock_recover};
use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

#[path = "pty_runtime_alias_closure.rs"]
mod closure;
#[path = "pty_runtime_alias_release.rs"]
mod release;
#[path = "pty_runtime_alias_release_retry.rs"]
mod release_retry;
pub(crate) use release::PtyRuntimeAliasReleasePoll;
pub(crate) use release::PtyRuntimeLocalAliasRelease;

pub(crate) const RUNTIME_ALIAS_STATE_GONE_DETAIL: &str =
    "PTY runtime disappeared before acknowledging master-alias closure";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PtyRuntimeAliasClosePoll {
    Pending,
    Closed,
}

pub(crate) struct PtyRuntimeAliasCustody {
    pub(crate) runtime: Mutex<Weak<WorkerState>>,
    pub(crate) release_handle: Mutex<Option<WorkerCustodyReleaseHandle>>,
    pub(crate) host_release: Mutex<release::HostReleaseState>,
    pub(crate) local_closed: AtomicBool,
    pub(crate) closure_runtime: tokio::runtime::Handle,
    pub(crate) durable_closure: Mutex<Option<Arc<DurableAliasClosure>>>,
    pub(crate) closure_task_started: AtomicBool,
    pub(crate) system_termination_committed: AtomicBool,
}

impl PtyRuntimeAliasCustody {
    pub(crate) fn new(closure_runtime: tokio::runtime::Handle) -> Self {
        Self {
            runtime: Mutex::new(Weak::new()),
            release_handle: Mutex::new(None),
            host_release: Mutex::new(release::HostReleaseState::Open),
            local_closed: AtomicBool::new(false),
            closure_runtime,
            durable_closure: Mutex::new(None),
            closure_task_started: AtomicBool::new(false),
            system_termination_committed: AtomicBool::new(false),
        }
    }

    #[cfg(test)]
    pub(crate) fn new_for_test() -> Self {
        Self::new(super::pty_runtime_alias_test_runtime::handle())
    }

    pub(crate) fn bind(&self, runtime: &Arc<WorkerState>) {
        *lock_recover(&self.runtime) = Arc::downgrade(runtime);
    }

    pub(crate) fn prepare_controller_release(
        &self,
        _permit: &ControllerMutationPermit,
    ) -> PreparedAliasClosure {
        self.prepare_alias_closure()
    }

    pub(crate) fn commit_controller_release(
        &self,
        _permit: &ControllerMutationPermit,
        prepared: PreparedAliasClosure,
    ) {
        prepared.commit(&self.durable_closure);
    }

    pub(crate) fn prepare_system_release(
        &self,
        _permit: &SystemMutationPermit,
    ) -> PreparedAliasClosure {
        self.prepare_alias_closure()
    }

    pub(crate) fn commit_system_release(
        &self,
        _permit: &SystemMutationPermit,
        prepared: PreparedAliasClosure,
    ) {
        prepared.commit(&self.durable_closure);
        self.system_termination_committed
            .store(true, Ordering::Release);
    }

    pub(crate) fn take_closure_authority(&self) -> Option<Arc<DurableAliasClosure>> {
        let authority = lock_recover(&self.durable_closure).clone()?;
        if self
            .closure_task_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return None;
        }
        Some(authority)
    }

    pub(crate) fn finish_closure_driver(&self) {
        self.closure_task_started.store(false, Ordering::Release);
    }

    pub(crate) fn system_termination_is_committed(&self) -> bool {
        self.system_termination_committed.load(Ordering::Acquire)
    }

    pub(crate) fn install_release_handle(&self, handle: WorkerCustodyReleaseHandle) {
        *lock_recover(&self.release_handle) = Some(handle);
    }

    pub(crate) fn clear_release_handle(&self, identity: ReadyWorkerCustodyIdentity) {
        let mut handle = lock_recover(&self.release_handle);
        if handle
            .as_ref()
            .is_some_and(|current| current.identity() == identity)
        {
            *handle = None;
        }
    }

    pub(crate) fn clear_all_release_handles(&self) {
        *lock_recover(&self.release_handle) = None;
    }

    pub(crate) fn released_successor_adoptable(&self) -> bool {
        self.local_closed.load(Ordering::Acquire)
            && matches!(
                &*lock_recover(&self.host_release),
                release::HostReleaseState::Retryable(_)
                    | release::HostReleaseState::Awaiting { .. }
                    | release::HostReleaseState::Finalized
            )
    }

    #[cfg(test)]
    pub(crate) fn release_identity_for_test(&self) -> Option<ReadyWorkerCustodyIdentity> {
        lock_recover(&self.release_handle)
            .as_ref()
            .map(WorkerCustodyReleaseHandle::identity)
    }

    #[cfg(test)]
    pub(crate) fn alias_closure_is_committed_for_test(&self) -> bool {
        lock_recover(&self.durable_closure).is_some()
    }

    pub(crate) fn close_local_aliases(&self) -> Result<PtyRuntimeAliasClosePoll> {
        if self.local_closed.load(Ordering::Acquire) {
            return Ok(PtyRuntimeAliasClosePoll::Closed);
        }
        let runtime = lock_recover(&self.runtime)
            .upgrade()
            .context(RUNTIME_ALIAS_STATE_GONE_DETAIL)?;
        let poll = runtime.poll_runtime_pty_alias_closure();
        if poll == PtyRuntimeAliasClosePoll::Pending {
            runtime.schedule_alias_closure();
        }
        if poll == PtyRuntimeAliasClosePoll::Closed {
            anyhow::ensure!(
                self.local_closed.load(Ordering::Acquire),
                RUNTIME_ALIAS_STATE_GONE_DETAIL
            );
        }
        Ok(poll)
    }

    pub(crate) fn prepare_alias_closure(&self) -> PreparedAliasClosure {
        let current = lock_recover(&self.durable_closure).clone();
        PreparedAliasClosure::capture(&self.closure_runtime, current)
    }

    pub(crate) fn mark_local_closed(&self) {
        self.local_closed.store(true, Ordering::Release);
    }
}
