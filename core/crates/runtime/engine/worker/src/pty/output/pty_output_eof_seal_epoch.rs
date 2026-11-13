use super::KernelPtyEofIdentity;
use crate::process::CommittedPtyReopenSealAuthority;
use std::fmt;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct CommittedKernelEofSealEpoch {
    pub(crate) identity: Arc<KernelPtyEofIdentity>,
    pub(crate) seal: CommittedPtyReopenSealAuthority,
}

impl fmt::Debug for CommittedKernelEofSealEpoch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl CommittedKernelEofSealEpoch {
    pub(crate) fn new(
        identity: Arc<KernelPtyEofIdentity>,
        seal: CommittedPtyReopenSealAuthority,
    ) -> Self {
        Self { identity, seal }
    }

    pub(crate) fn identity(&self) -> &Arc<KernelPtyEofIdentity> {
        &self.identity
    }

    pub(crate) fn seal(&self) -> &CommittedPtyReopenSealAuthority {
        &self.seal
    }

    pub(crate) fn is_current(&self, expected: &Arc<KernelPtyEofIdentity>) -> bool {
        Arc::ptr_eq(&self.identity, expected)
    }

    pub(crate) fn same_authority(&self, other: &Self) -> bool {
        self.is_current(&other.identity) && self.seal.same_authority(&other.seal)
    }
}
