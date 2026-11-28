use super::*;

impl PowerLossProcessTree {
    pub(in super::super::super) fn capture(
        terminal_id: Uuid,
        root: RuntimeProcessIdentity,
    ) -> Result<Self> {
        let tree = ResilienceProcessTree::capture(root)
            .map_err(|error| anyhow::anyhow!("{TREE_CAPTURE_CONTEXT}: {error:#}"))?;
        ensure!(tree.identities().contains(&root), TREE_ROOT_MISSING_DETAIL);
        Ok(Self {
            terminal_id,
            root,
            state: Some(RetainedTreeState::Captured(tree)),
            #[cfg(target_os = "linux")]
            rollback: None,
        })
    }

    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    pub(in super::super::super) fn capture_custody(
        terminal_id: Uuid,
        root: RuntimeProcessIdentity,
        custody: yolo_process_test_support::PtyCustodyRecoveryAuthority,
    ) -> Result<Self> {
        let tree = ResilienceProcessTree::capture_custody(root, custody)
            .map_err(|error| anyhow::anyhow!("{TREE_CAPTURE_CONTEXT}: {error:#}"))?;
        ensure!(tree.identities().contains(&root), TREE_ROOT_MISSING_DETAIL);
        Ok(Self {
            terminal_id,
            root,
            state: Some(RetainedTreeState::Captured(tree)),
        })
    }
}
