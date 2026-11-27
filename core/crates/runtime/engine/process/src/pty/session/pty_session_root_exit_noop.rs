use super::PtySessionProof;
use anyhow::Result;

impl PtySessionProof {
    pub(in crate::process) fn confirm_process_capability_root_exit(
        &self,
        _root: &super::super::LiveProcessIdentity,
    ) -> Result<()> {
        Ok(())
    }
}
