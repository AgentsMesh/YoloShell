use super::*;

impl PowerLossProcessTree {
    pub(in super::super::super) fn capture_linux(
        bound: &super::super::super::BoundRuntimeRecord,
    ) -> Result<Self> {
        let (tree, rollback) = bound.capture_linux_power_loss_tree()?;
        Ok(Self {
            terminal_id: bound.record().terminal_id,
            root: super::super::super::runtime_child(bound),
            state: Some(RetainedTreeState::Captured(tree)),
            rollback: Some(rollback),
        })
    }
}
