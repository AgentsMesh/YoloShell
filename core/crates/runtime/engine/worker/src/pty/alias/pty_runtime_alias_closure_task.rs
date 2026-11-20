use super::pty_runtime_alias_custody::PtyRuntimeAliasClosePoll;
use super::pty_runtime_alias_custody::PtyRuntimeAliasCustody;
use super::{ControllerMutationPermit, WorkerState, lock_recover};
use std::sync::Arc;

pub(crate) struct DurableAliasClosure {
    pub(crate) runtime: tokio::runtime::Handle,
}

pub(crate) struct PreparedAliasClosure {
    pub(crate) authority: Arc<DurableAliasClosure>,
    pub(crate) install: bool,
}

pub(crate) struct AliasClosureDriver {
    custody: Arc<PtyRuntimeAliasCustody>,
}

impl PreparedAliasClosure {
    pub(crate) fn capture(
        runtime: &tokio::runtime::Handle,
        current: Option<Arc<DurableAliasClosure>>,
    ) -> Self {
        let (authority, install) = match current {
            Some(authority) => (authority, false),
            None => (
                Arc::new(DurableAliasClosure {
                    runtime: runtime.clone(),
                }),
                true,
            ),
        };
        Self { authority, install }
    }

    pub(crate) fn commit(self, destination: &std::sync::Mutex<Option<Arc<DurableAliasClosure>>>) {
        if self.install {
            let mut destination = lock_recover(destination);
            debug_assert!(destination.is_none());
            *destination = Some(self.authority);
        }
    }
}

impl WorkerState {
    pub(crate) fn prepare_controller_alias_release(
        &self,
        permit: &ControllerMutationPermit,
    ) -> PreparedAliasClosure {
        self.pty_alias_custody.prepare_controller_release(permit)
    }

    pub(crate) fn commit_controller_alias_release(
        self: &Arc<Self>,
        permit: &ControllerMutationPermit,
        prepared: PreparedAliasClosure,
    ) {
        self.pty_alias_custody
            .commit_controller_release(permit, prepared);
        self.fence_successor_alias_exports();
    }

    pub(crate) fn prepare_system_alias_release(
        &self,
        permit: &super::SystemMutationPermit,
    ) -> PreparedAliasClosure {
        self.pty_alias_custody.prepare_system_release(permit)
    }

    pub(crate) fn commit_system_alias_release(
        &self,
        permit: &super::SystemMutationPermit,
        prepared: PreparedAliasClosure,
    ) {
        self.pty_alias_custody
            .commit_system_release(permit, prepared);
        self.fence_successor_alias_exports();
    }

    pub(crate) fn fence_successor_alias_exports(&self) {
        if let Some(source) = lock_recover(&self.secondary_custody).as_ref() {
            source.fence_successor_exports();
        }
    }

    pub(crate) fn schedule_alias_closure(self: &Arc<Self>) {
        let Some(authority) = self.pty_alias_custody.take_closure_authority() else {
            return;
        };
        let state = Arc::clone(self);
        let driver = AliasClosureDriver {
            custody: Arc::clone(&self.pty_alias_custody),
        };
        let runtime = authority.runtime.clone();
        runtime.spawn(async move {
            let _driver = driver;
            state.drive_alias_closure(authority).await;
        });
    }

    pub(crate) async fn drive_alias_closure(self: Arc<Self>, authority: Arc<DurableAliasClosure>) {
        loop {
            let permit = self.admit_alias_closure_mutation(&authority).await;
            let poll = self.close_runtime_pty_aliases_task_admitted(&permit);
            let source = match poll {
                PtyRuntimeAliasClosePoll::Pending => lock_recover(&self.secondary_custody).clone(),
                PtyRuntimeAliasClosePoll::Closed => None,
            };
            drop(permit);
            match (poll, source) {
                (PtyRuntimeAliasClosePoll::Closed, _) => return,
                (PtyRuntimeAliasClosePoll::Pending, Some(source)) => {
                    source.wait_for_successor_export_progress().await;
                }
                (PtyRuntimeAliasClosePoll::Pending, None) => {
                    tokio::time::sleep(crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL)
                        .await;
                }
            }
        }
    }
}

impl Drop for AliasClosureDriver {
    fn drop(&mut self) {
        self.custody.finish_closure_driver();
    }
}
