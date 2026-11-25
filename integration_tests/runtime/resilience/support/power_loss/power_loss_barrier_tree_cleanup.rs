use super::{
    PowerLossProcessTree, RetainedTreeState, TREE_STATE_INVALID_DETAIL, TREE_STATE_MISSING_DETAIL,
};
use anyhow::Result;

impl PowerLossProcessTree {
    pub(in super::super) fn kill_cleanup_can_commit(&self) -> bool {
        match self.state() {
            RetainedTreeState::Captured(_) => false,
            RetainedTreeState::Freezing(tree) => tree.kill_cleanup_is_armed(),
            RetainedTreeState::Frozen(_) => true,
            RetainedTreeState::Killing(tree) => tree.kill_cleanup_is_armed(),
            RetainedTreeState::Gone => true,
        }
    }

    pub(in super::super) fn commit_kill_cleanup(&mut self) -> Result<()> {
        match self.state.as_mut().expect(TREE_STATE_MISSING_DETAIL) {
            RetainedTreeState::Frozen(tree) => tree.arm_kill_cleanup(),
            RetainedTreeState::Freezing(tree) if tree.kill_cleanup_is_armed() => {}
            RetainedTreeState::Killing(tree) if tree.kill_cleanup_is_armed() => {}
            RetainedTreeState::Gone => {}
            _ => anyhow::bail!(TREE_STATE_INVALID_DETAIL),
        }
        #[cfg(target_os = "linux")]
        if let Some(rollback) = &mut self.rollback {
            rollback.commit_to_kill();
        }
        Ok(())
    }

    pub(in super::super) fn kill_cleanup_is_armed(&self) -> bool {
        match self.state() {
            RetainedTreeState::Freezing(tree) => tree.kill_cleanup_is_armed(),
            RetainedTreeState::Frozen(tree) => tree.kill_cleanup_is_armed(),
            RetainedTreeState::Killing(tree) => tree.kill_cleanup_is_armed(),
            RetainedTreeState::Captured(_) | RetainedTreeState::Gone => false,
        }
    }
}
