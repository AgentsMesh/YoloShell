use super::{LiveProcessIdentityKey, RootWaitAuthority, UnreapedRootExit, is_wait_authority_loss};
use crate::process::{DirectChildExitStatus, ProcessIdentity};
use anyhow::{Result, anyhow, ensure};
use std::fmt;

const ROOT_GONE_PROOF_MISMATCH_DETAIL: &str =
    "lost PTY root proof does not match retained wait authority";

pub struct WaitAuthorityLoss {
    _private: (),
}

pub struct LostRootWaitAuthority {
    authority: RootWaitAuthority,
}

pub struct RootGoneWithoutWait {
    root: ProcessIdentity,
    live_root: LiveProcessIdentityKey,
}

pub struct RootWaitReapError {
    authority: RootWaitAuthority,
    source: anyhow::Error,
}

struct RetainedRootWaitError {
    _authority: RootWaitAuthority,
    source: anyhow::Error,
}

impl WaitAuthorityLoss {
    pub fn from_error(error: &anyhow::Error) -> Option<Self> {
        is_wait_authority_loss(error).then_some(Self { _private: () })
    }
}

impl RootWaitAuthority {
    pub fn lose(self, _proof: WaitAuthorityLoss) -> LostRootWaitAuthority {
        LostRootWaitAuthority { authority: self }
    }

    pub fn reap_preserving_authority(
        self,
        proof: UnreapedRootExit,
    ) -> std::result::Result<DirectChildExitStatus, RootWaitReapError> {
        let mut authority = self;
        let result = authority.wait_exited(proof);
        result.map_err(|source| RootWaitReapError { authority, source })
    }
}

impl LostRootWaitAuthority {
    pub fn observe_gone(&self) -> Result<Option<RootGoneWithoutWait>> {
        RootGoneWithoutWait::observe_exact_generation(
            self.authority.root,
            &self.authority.live_root,
        )
    }

    pub fn validate(&self, proof: &RootGoneWithoutWait) -> Result<()> {
        ensure!(
            proof.identities() == (self.authority.root, self.authority.live_root.key()),
            ROOT_GONE_PROOF_MISMATCH_DETAIL
        );
        Ok(())
    }
}

impl RootGoneWithoutWait {
    pub(in crate::process) fn observe_exact_generation(
        root: ProcessIdentity,
        live_root: &super::LiveProcessIdentity,
    ) -> Result<Option<Self>> {
        if live_root.try_matches()? {
            return Ok(None);
        }
        Ok(Some(Self {
            root,
            live_root: live_root.key(),
        }))
    }

    pub(in crate::process) fn identities(&self) -> (ProcessIdentity, LiveProcessIdentityKey) {
        (self.root, self.live_root)
    }
}

impl RootWaitReapError {
    pub fn error(&self) -> &anyhow::Error {
        &self.source
    }

    pub fn into_parts(self) -> (RootWaitAuthority, anyhow::Error) {
        (self.authority, self.source)
    }

    pub(super) fn into_retained_error(self) -> anyhow::Error {
        anyhow!(RetainedRootWaitError {
            _authority: self.authority,
            source: self.source,
        })
    }
}

impl fmt::Debug for RetainedRootWaitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple(stringify!(RetainedRootWaitError))
            .field(&self.source)
            .finish()
    }
}

impl fmt::Display for RetainedRootWaitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.source.fmt(formatter)
    }
}

impl std::error::Error for RetainedRootWaitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/unreaped/unreaped_root_exit_loss_tests.rs"]
mod tests;
