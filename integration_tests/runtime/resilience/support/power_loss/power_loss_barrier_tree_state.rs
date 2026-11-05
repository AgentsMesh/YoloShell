use super::super::identity_key;
use anyhow::{Result, ensure};
use uuid::Uuid;
use yolo_process_test_support::{
    ProcessIdentity as RuntimeProcessIdentity, ResilienceFreezePoll, ResilienceFreezingProcessTree,
    ResilienceFrozenProcessTree, ResilienceKillCommitPoll, ResilienceKillPendingProcessTree,
    ResilienceKillPoll, ResilienceProcessTree,
};

const TREE_CAPTURE_CONTEXT: &str = "capture retained simulated power-loss process tree";
const TREE_ROOT_MISSING_DETAIL: &str = "retained process-tree closure omitted its root";
const TREE_STATE_MISSING_DETAIL: &str = "retained process-tree state is missing";
const TREE_STATE_INVALID_DETAIL: &str = "retained process-tree transition is invalid";

#[path = "power_loss_barrier_tree_diagnostic.rs"]
mod diagnostic;

enum RetainedTreeState {
    Captured(ResilienceProcessTree),
    Freezing(ResilienceFreezingProcessTree),
    Frozen(ResilienceFrozenProcessTree),
    Killing(ResilienceKillPendingProcessTree),
    Gone,
}

pub(super) enum TreeKillPoll {
    Refreeze,
    Pending,
    Gone,
}

pub(in super::super) struct PowerLossProcessTree {
    terminal_id: Uuid,
    root: RuntimeProcessIdentity,
    state: Option<RetainedTreeState>,
    #[cfg(target_os = "linux")]
    rollback: Option<yolo_process_test_support::linux_containment::ResilienceContainmentRollback>,
}

impl PowerLossProcessTree {
    pub(in super::super) fn key(&self) -> (u32, u64) {
        identity_key(self.root)
    }

    pub(crate) fn terminal_id(&self) -> Uuid {
        self.terminal_id
    }

    pub(in super::super) fn identities(&self) -> Vec<RuntimeProcessIdentity> {
        match self.state() {
            RetainedTreeState::Captured(tree) => tree.identities(),
            RetainedTreeState::Freezing(tree) => tree.identities(),
            RetainedTreeState::Frozen(tree) => tree.identities(),
            RetainedTreeState::Killing(tree) => tree.identities(),
            RetainedTreeState::Gone => Vec::new(),
        }
    }

    pub(super) fn is_gone(&self) -> bool {
        matches!(self.state(), RetainedTreeState::Gone)
    }

    pub(in super::super) fn is_frozen(&self) -> bool {
        #[cfg(target_os = "linux")]
        if self
            .rollback
            .as_ref()
            .is_some_and(|rollback| !rollback.is_frozen().unwrap_or(false))
        {
            return false;
        }
        matches!(self.state(), RetainedTreeState::Frozen(_))
    }

    pub(super) fn poll_freeze(&mut self) -> Result<bool> {
        #[cfg(target_os = "linux")]
        if matches!(self.state(), RetainedTreeState::Captured(_))
            && let Some(rollback) = &mut self.rollback
        {
            rollback.begin_freeze()?;
        }
        match self.take_state() {
            RetainedTreeState::Captured(tree) => {
                self.state = Some(RetainedTreeState::Freezing(tree.begin_force()));
                self.poll_freeze()
            }
            RetainedTreeState::Freezing(tree) => match tree.poll() {
                Ok(ResilienceFreezePoll::Pending(tree)) => {
                    self.state = Some(RetainedTreeState::Freezing(tree));
                    Ok(false)
                }
                Ok(ResilienceFreezePoll::Frozen(tree)) => {
                    self.state = Some(RetainedTreeState::Frozen(tree));
                    Ok(true)
                }
                Err(error) => {
                    let (tree, source) = error.into_parts();
                    self.state = Some(RetainedTreeState::Freezing(tree));
                    Err(source)
                }
            },
            state @ RetainedTreeState::Frozen(_) => {
                self.state = Some(state);
                Ok(true)
            }
            state => self.invalid(state),
        }
    }

    pub(super) fn commit_kill(&mut self, commit_fence: impl FnMut() -> Result<()>) -> Result<bool> {
        match self.take_state() {
            RetainedTreeState::Frozen(tree) => {
                match tree.kill_with_destructive_commit_fence(commit_fence) {
                    Ok(ResilienceKillCommitPoll::Refreeze(tree)) => {
                        self.state = Some(RetainedTreeState::Freezing(tree));
                        Ok(false)
                    }
                    Ok(ResilienceKillCommitPoll::Pending(tree)) => {
                        self.state = Some(RetainedTreeState::Killing(tree));
                        Ok(true)
                    }
                    Err(error) => {
                        let (tree, source) = error.into_parts();
                        self.state = Some(RetainedTreeState::Frozen(tree));
                        Err(source)
                    }
                }
            }
            state @ RetainedTreeState::Killing(_) => {
                self.state = Some(state);
                Ok(true)
            }
            state => self.invalid(state),
        }
    }

    pub(super) fn poll_kill(&mut self) -> Result<TreeKillPoll> {
        match self.take_state() {
            RetainedTreeState::Killing(tree) => match tree.poll() {
                Ok(ResilienceKillPoll::Refreeze(tree)) => {
                    self.state = Some(RetainedTreeState::Freezing(tree));
                    Ok(TreeKillPoll::Refreeze)
                }
                Ok(ResilienceKillPoll::Pending(tree)) => {
                    self.state = Some(RetainedTreeState::Killing(tree));
                    Ok(TreeKillPoll::Pending)
                }
                Ok(ResilienceKillPoll::Gone(_)) => {
                    self.state = Some(RetainedTreeState::Gone);
                    Ok(TreeKillPoll::Gone)
                }
                Err(error) => {
                    let (tree, source) = error.into_parts();
                    self.state = Some(RetainedTreeState::Killing(tree));
                    Err(source)
                }
            },
            RetainedTreeState::Gone => {
                self.state = Some(RetainedTreeState::Gone);
                Ok(TreeKillPoll::Gone)
            }
            state => self.invalid(state),
        }
    }

    fn invalid<T>(&mut self, state: RetainedTreeState) -> Result<T> {
        self.state = Some(state);
        anyhow::bail!(TREE_STATE_INVALID_DETAIL)
    }

    fn state(&self) -> &RetainedTreeState {
        self.state.as_ref().expect(TREE_STATE_MISSING_DETAIL)
    }

    fn take_state(&mut self) -> RetainedTreeState {
        self.state.take().expect(TREE_STATE_MISSING_DETAIL)
    }
}

#[path = "power_loss_barrier_tree_capture.rs"]
mod capture;
#[path = "power_loss_barrier_tree_cleanup.rs"]
mod cleanup;
#[cfg(target_os = "linux")]
#[path = "power_loss_barrier_tree_linux.rs"]
mod linux;
