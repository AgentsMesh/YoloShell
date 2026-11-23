use super::{KernelEofCommitBinding, KernelEofValidationAuthority, ValidatedKernelEof};
use crate::process::CommittedPtyReopenSealAuthority;
use std::sync::Arc;

pub struct ValidatedSealedKernelEof {
    eof: Arc<super::KernelEofBinding>,
    seal: CommittedPtyReopenSealAuthority,
}

impl KernelEofValidationAuthority {
    pub fn bind_committed_reopen_seal(
        &self,
        validated: ValidatedKernelEof,
        seal: &CommittedPtyReopenSealAuthority,
    ) -> Option<ValidatedSealedKernelEof> {
        self.0
            .same_authority(&validated.0)
            .then(|| ValidatedSealedKernelEof {
                eof: validated.0,
                seal: seal.clone(),
            })
    }
}

impl KernelEofCommitBinding {
    pub(crate) fn consume_sealed_token(
        &self,
        validated: ValidatedSealedKernelEof,
    ) -> Option<CommittedPtyReopenSealAuthority> {
        self.0
            .same_authority(&validated.eof)
            .then_some(validated.seal)
    }

    pub(crate) fn consume_sealed(
        &self,
        seal: &CommittedPtyReopenSealAuthority,
        validated: ValidatedSealedKernelEof,
    ) -> bool {
        self.consume_sealed_token(validated)
            .is_some_and(|validated_seal| seal.same_authority(&validated_seal))
    }
}

#[cfg(any(test, feature = "runtime-component-tests"))]
impl ValidatedKernelEof {
    pub(crate) fn bind_reopen_seal_for_test(
        self,
        seal: &CommittedPtyReopenSealAuthority,
    ) -> ValidatedSealedKernelEof {
        ValidatedSealedKernelEof {
            eof: self.0,
            seal: seal.clone(),
        }
    }
}
