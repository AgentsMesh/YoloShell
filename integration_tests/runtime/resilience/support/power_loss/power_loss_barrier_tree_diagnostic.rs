use super::{PowerLossProcessTree, RetainedTreeState};

impl PowerLossProcessTree {
    pub(crate) fn authority_snapshot(&self) -> String {
        #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
        {
            return match self.state() {
                RetainedTreeState::Captured(tree) => tree.authority_snapshot(),
                RetainedTreeState::Freezing(tree) => tree.authority_snapshot(),
                RetainedTreeState::Frozen(tree) => tree.authority_snapshot(),
                RetainedTreeState::Killing(tree) => tree.authority_snapshot(),
                RetainedTreeState::Gone => String::new(),
            };
        }
        #[cfg(not(all(target_os = "macos", feature = "adversarial-test-hooks")))]
        String::new()
    }
}
