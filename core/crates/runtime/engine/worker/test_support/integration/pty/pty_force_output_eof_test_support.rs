use crate::worker::pty_output_progress::KernelEofCommitOutcome;

impl ScriptedForceOutput {
    pub(in crate::worker) fn apply_kernel_eof_outcome(
        &self,
        authority: &mut KernelPtyEofAuthority,
        outcome: KernelEofCommitOutcome,
    ) -> bool {
        match outcome {
            KernelEofCommitOutcome::Committed => {
                authority.retire_validation();
                true
            }
            KernelEofCommitOutcome::OutputInvalidated => {
                self.kernel_eof.revoke_kernel_eof_for_test();
                authority.retire_validation();
                false
            }
            KernelEofCommitOutcome::Rejected => false,
        }
    }

    pub(in crate::worker) fn validate_kernel_eof_authority(
        &mut self,
        authority: &KernelPtyEofAuthority,
        version: &ProcessClosureVersion,
    ) -> Result<Option<crate::process::kernel_eof_authority::ValidatedKernelEof>> {
        self.final_eof_revalidation_calls += OPERATION_CALL_INCREMENT;
        let fault = self.final_eof_revalidation_fault.take();
        if matches!(&fault, Some(FinalEofRevalidationFault::Revoke)) {
            self.kernel_eof.revoke_kernel_eof_for_test();
        }
        if !self.validates_kernel_eof_authority(authority, version)? {
            return Ok(None);
        }
        let validated = authority.validate_scripted_reader(version, &self.kernel_eof);
        if validated.is_some() {
            self.final_eof_token_mints += OPERATION_CALL_INCREMENT;
        }
        match fault {
            Some(FinalEofRevalidationFault::Error(detail)) => bail!(detail),
            Some(FinalEofRevalidationFault::Panic(detail)) => panic!("{detail}"),
            Some(FinalEofRevalidationFault::Revoke) | None => Ok(validated),
        }
    }

    fn assert_final_eof_revalidation_reached(&self) {
        assert_eq!(self.final_eof_revalidation_calls, OPERATION_CALL_INCREMENT);
    }

    fn assert_final_eof_token_minted(&self) {
        assert_eq!(self.final_eof_token_mints, OPERATION_CALL_INCREMENT);
    }

    fn assert_final_eof_token_not_minted(&self) {
        assert_eq!(self.final_eof_token_mints, usize::default());
    }

    fn assert_kernel_eof_revoked(&self, detail: &str) {
        assert!(self.kernel_eof.kernel_eof_identity().is_none(), "{detail}");
    }
}
