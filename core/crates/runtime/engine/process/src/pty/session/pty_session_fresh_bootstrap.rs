use super::PtySessionProof;
use anyhow::Result;

impl PtySessionProof {
    pub fn capture_fresh_bootstrap_bound(
        master: &dyn portable_pty::MasterPty,
    ) -> Result<(Self, uuid::Uuid)> {
        #[cfg(target_os = "linux")]
        let issuer = crate::process::ProcessIdentity::capture(std::process::id())?;
        let token = uuid::Uuid::new_v4();
        let mut proof = Self::capture_bound(master, token.to_string())?;
        #[cfg(target_os = "linux")]
        {
            proof.bootstrap.linux_origin = Some(issuer);
        }
        proof.bootstrap.exec_fenced = true;
        proof.bootstrap.capability_covered = false;
        proof.bootstrap_exec = super::super::bootstrap_exec::BootstrapExecState::armed();
        Ok((proof, token))
    }
}
