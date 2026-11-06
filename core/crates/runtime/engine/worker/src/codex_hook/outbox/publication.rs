use crate::agent::hooks::{ApplyDisposition, NormalizedLifecycle, ProcessExitDisposition, Reducer};
use std::ops::Deref;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::worker) struct CodexLifecycleView {
    lifecycle: NormalizedLifecycle,
    authority_revision: u64,
}

impl CodexLifecycleView {
    pub(in crate::worker::codex_hook) fn capture(reducer: &Reducer) -> Self {
        Self {
            lifecycle: reducer.state().clone(),
            authority_revision: reducer.revision(),
        }
    }

    pub(in crate::worker) fn authority_revision(&self) -> u64 {
        self.authority_revision
    }
}

impl Deref for CodexLifecycleView {
    type Target = NormalizedLifecycle;

    fn deref(&self) -> &Self::Target {
        &self.lifecycle
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::worker) enum CodexAuthorityView {
    Active(CodexLifecycleView),
    Exhausted(CodexLifecycleView),
    AuthorityLost,
    StorageUnavailable,
    Superseded,
}

#[derive(Clone, Debug)]
pub(super) struct CodexHookPublication {
    pub(super) disposition: ApplyDisposition,
    pub(super) state: NormalizedLifecycle,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::worker) struct ReplayReport {
    pub(in crate::worker) applied: usize,
    pub(in crate::worker) replayed: usize,
    pub(in crate::worker) stale: usize,
}

#[derive(Clone, Debug)]
pub(super) struct CodexProcessExitPublication {
    pub(super) disposition: ProcessExitDisposition,
    pub(super) replay: ReplayReport,
}
