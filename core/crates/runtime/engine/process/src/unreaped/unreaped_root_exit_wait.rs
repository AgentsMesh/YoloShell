use super::{DirectChildExitStatus, RootWaitAuthority, UnreapedRootExit};
use anyhow::{Result, ensure};

impl RootWaitAuthority {
    pub fn wait_exited(&mut self, proof: UnreapedRootExit) -> Result<DirectChildExitStatus> {
        ensure!(
            proof.identities() == (self.root, self.live_root.key()),
            super::ROOT_EXIT_PROOF_MISMATCH_DETAIL
        );
        if let Some(status) = self.waited_status {
            return Ok(status);
        }
        let status = super::super::wait_direct_child(self.root.pid)?;
        self.waited_status = Some(status);
        Ok(status)
    }

    pub fn is_waited(&self) -> bool {
        self.waited_status.is_some()
    }
}
