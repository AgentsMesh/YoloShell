use super::{CapturedProcessTree, FreezingProcessTree, FrozenProcessTree, KillPendingProcessTree};
use crate::process::{LiveProcessIdentity, ProcessIdentity};

type RetainedProcessTarget = (LiveProcessIdentity, ProcessIdentity);

impl CapturedProcessTree {
    pub(in crate::process) fn resilience_retained_targets(&self) -> Vec<RetainedProcessTarget> {
        self.captured_targets()
    }
}

macro_rules! delegate_retained_targets {
    ($state:ty) => {
        impl $state {
            pub(in crate::process) fn resilience_retained_targets(
                &self,
            ) -> Vec<RetainedProcessTarget> {
                self.tree.resilience_retained_targets()
            }
        }
    };
}

delegate_retained_targets!(FreezingProcessTree);
delegate_retained_targets!(FrozenProcessTree);
delegate_retained_targets!(KillPendingProcessTree);
