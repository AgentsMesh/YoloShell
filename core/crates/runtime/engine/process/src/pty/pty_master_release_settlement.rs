use super::{
    PtyClosureIdentity, PtyMasterAliasReleaseAuthority, PtyMasterAliasesClosed,
    PtyMasterAliasesClosedIdentity,
};
use anyhow::{Result, ensure};
use std::sync::Arc;
use std::sync::atomic::Ordering;

const HOST_RELEASE_IDENTITY_MISMATCH_DETAIL: &str =
    "Host PTY custody release acknowledgement belongs to another local closure";
const LOCAL_ALIAS_PROOF_ALREADY_CONSUMED_DETAIL: &str =
    "local PTY master-alias closure proof was already consumed";

pub struct PtyLocalMasterAliasesClosed {
    identity: Arc<PtyClosureIdentity>,
}

#[derive(Clone)]
pub struct PtyHostReleaseVerifier {
    pub(super) identity: Arc<PtyClosureIdentity>,
}

pub struct PtyHostReleasedAck {
    identity: Arc<PtyClosureIdentity>,
}

pub struct PtyMasterAliasClosureSettlement {
    identity: Arc<PtyClosureIdentity>,
}

impl PtyMasterAliasReleaseAuthority {
    pub fn take_local_aliases_closed(
        &self,
    ) -> Result<(PtyLocalMasterAliasesClosed, PtyHostReleaseVerifier)> {
        ensure!(
            self.local_proof_available
                .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
                .is_ok(),
            LOCAL_ALIAS_PROOF_ALREADY_CONSUMED_DETAIL
        );
        Ok((
            PtyLocalMasterAliasesClosed {
                identity: Arc::clone(&self.identity),
            },
            PtyHostReleaseVerifier {
                identity: Arc::clone(&self.identity),
            },
        ))
    }
}

impl PtyHostReleaseVerifier {
    #[cfg(not(target_os = "macos"))]
    pub fn release_control(
        &self,
        offer: &crate::pty_custody::CustodyOffer,
    ) -> Result<crate::pty_custody::CustodyControl> {
        crate::pty_custody::CustodyControl::release(offer)
    }

    pub fn acknowledge(
        self,
        _exact: crate::pty_custody::ExactCustodyReleasedAck,
    ) -> PtyHostReleasedAck {
        self.acknowledgement()
    }

    pub fn acknowledge_holder_gone(
        self,
        _exact: crate::pty_custody::ExactCustodyHolderGone,
    ) -> PtyHostReleasedAck {
        self.acknowledgement()
    }

    fn acknowledgement(self) -> PtyHostReleasedAck {
        PtyHostReleasedAck {
            identity: self.identity,
        }
    }
}

impl PtyLocalMasterAliasesClosed {
    pub fn finalize_host_release(
        self,
        acknowledgement: PtyHostReleasedAck,
    ) -> std::result::Result<
        PtyMasterAliasClosureSettlement,
        (PtyLocalMasterAliasesClosed, anyhow::Error),
    > {
        if !Arc::ptr_eq(&self.identity, &acknowledgement.identity) {
            return Err((self, anyhow::anyhow!(HOST_RELEASE_IDENTITY_MISMATCH_DETAIL)));
        }
        Ok(PtyMasterAliasClosureSettlement {
            identity: self.identity,
        })
    }

    pub fn finalize_without_secondary_custody(self) -> PtyMasterAliasClosureSettlement {
        PtyMasterAliasClosureSettlement {
            identity: self.identity,
        }
    }
}

impl PtyMasterAliasClosureSettlement {
    pub fn mint_final_proof(self) -> PtyMasterAliasesClosed {
        PtyMasterAliasesClosed {
            identity: PtyMasterAliasesClosedIdentity::Closure(self.identity),
            #[cfg(any(test, feature = "runtime-component-tests"))]
            drop_panic: None,
        }
    }
}
