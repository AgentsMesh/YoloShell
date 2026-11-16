use super::PtySessionProof;
use super::macos_object_identity::MacosPtyObjectIdentity;
use super::macos_passive_master::MacosPassivePtyMaster;
use anyhow::Result;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
use anyhow::ensure;
use std::path::Path;

const MASTER_AUTHORITY_MISMATCH: &str = "PTY reopen-seal master authority changed";

pub(super) enum MacosPtyReopenMasterAuthority {
    Local(MacosPassivePtyMaster),
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    Custody(crate::process::PtyMasterFdCustodyProbe),
}

impl MacosPtyReopenMasterAuthority {
    pub(super) fn capture(proof: &PtySessionProof) -> Result<Self> {
        #[cfg(any(test, feature = "adversarial-test-hooks"))]
        if let Some(custody) = proof.custody_probe.as_ref() {
            ensure!(proof.is_custody_pty_object(), MASTER_AUTHORITY_MISMATCH);
            custody.confirm_exact()?;
            return Ok(Self::Custody(custody.clone()));
        }
        MacosPassivePtyMaster::duplicate_exact(proof).map(Self::Local)
    }

    pub(super) fn slave_path(&self) -> &Path {
        match self {
            Self::Local(master) => master.slave_path(),
            #[cfg(any(test, feature = "adversarial-test-hooks"))]
            Self::Custody(custody) => custody.pty_path(),
        }
    }

    pub(super) fn object_identity(&self) -> MacosPtyObjectIdentity {
        match self {
            Self::Local(master) => master.object_identity(),
            #[cfg(any(test, feature = "adversarial-test-hooks"))]
            Self::Custody(custody) => custody.pty_object_identity(),
        }
    }

    pub(super) fn revalidate(&self, proof: &PtySessionProof) -> Result<()> {
        match self {
            Self::Local(master) => master.revalidate_exact(proof),
            #[cfg(any(test, feature = "adversarial-test-hooks"))]
            Self::Custody(custody) => {
                ensure!(
                    proof.is_custody_pty_object()
                        && proof.custody_probe.as_ref() == Some(custody)
                        && proof.tty_path == custody.pty_path()
                        && proof.tty_object_identity == Some(custody.pty_object_identity()),
                    MASTER_AUTHORITY_MISMATCH
                );
                custody.confirm_exact()
            }
        }
    }

    pub(super) fn passive(&self) -> Option<&MacosPassivePtyMaster> {
        match self {
            Self::Local(master) => Some(master),
            #[cfg(any(test, feature = "adversarial-test-hooks"))]
            Self::Custody(_) => None,
        }
    }
}
