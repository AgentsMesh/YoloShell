use super::{FreezingProcessTree, FrozenProcessTree, KillPendingProcessTree};
use crate::process::{ProcessIdentity, RootGoneWithoutWait, UnreapedRootExit};
use anyhow::Result;

macro_rules! delegate_tree_authority {
    ($state:ty) => {
        impl $state {
            pub fn root(&self) -> ProcessIdentity {
                self.tree.root()
            }

            pub fn mark_root_exited_unreaped(&mut self, proof: UnreapedRootExit) -> Result<()> {
                self.tree.mark_root_exited_unreaped(proof)
            }

            pub fn mark_root_gone_without_wait(
                &mut self,
                proof: RootGoneWithoutWait,
            ) -> Result<()> {
                self.tree.mark_root_gone_without_wait(proof)
            }

            pub fn mark_ownership_fault(&mut self) {
                self.tree.mark_ownership_fault();
            }
        }
    };
}

delegate_tree_authority!(FreezingProcessTree);
delegate_tree_authority!(FrozenProcessTree);
delegate_tree_authority!(KillPendingProcessTree);
