use super::PtyRuntimeAliasClosePoll;
use crate::worker::{AliasClosureMutationPermit, WorkerState, lock_recover};
use std::sync::Arc;
use std::sync::atomic::Ordering;

impl WorkerState {
    pub(crate) fn poll_runtime_pty_alias_closure(self: &Arc<Self>) -> PtyRuntimeAliasClosePoll {
        #[cfg(test)]
        self.control_test_hooks
            .alias_closure_before_try_admission
            .pause_if_armed();
        if let Ok(permit) = Arc::clone(&self.mutation_gate).try_lock_owned() {
            let poll = self.close_runtime_pty_aliases_admitted();
            drop(permit);
            return poll;
        }
        PtyRuntimeAliasClosePoll::Pending
    }

    pub(crate) fn close_runtime_pty_aliases_task_admitted(
        &self,
        _permit: &AliasClosureMutationPermit,
    ) -> PtyRuntimeAliasClosePoll {
        self.close_runtime_pty_aliases_admitted()
    }

    fn close_runtime_pty_aliases_admitted(&self) -> PtyRuntimeAliasClosePoll {
        if self.pty_alias_custody.local_closed.load(Ordering::Acquire) {
            return PtyRuntimeAliasClosePoll::Closed;
        }
        let mut secondary = lock_recover(&self.secondary_custody);
        if secondary
            .as_ref()
            .is_some_and(|source| !source.fence_successor_exports())
        {
            return PtyRuntimeAliasClosePoll::Pending;
        }
        if let Some(source) = secondary.as_ref() {
            source.close_platform_after_successor_exports();
        }
        drop(secondary.take());
        drop(secondary);
        drop(lock_recover(&self.writer).take());
        drop(lock_recover(&self.master).take());
        self.pty_alias_custody.mark_local_closed();
        PtyRuntimeAliasClosePoll::Closed
    }
}

impl Drop for WorkerState {
    fn drop(&mut self) {
        let _ = self.close_runtime_pty_aliases_admitted();
    }
}
