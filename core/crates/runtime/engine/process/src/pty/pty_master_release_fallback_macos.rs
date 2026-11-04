use super::{PtyClosureIdentity, PtyHostReleaseVerifier, PtyMasterAliasVerifier};
use crate::pty_custody::{
    CustodyControl, CustodyOffer, CustodyReleaseSettlement, CustodyRuntimeIdentity,
};
use crate::sha256_digest::Sha256Digest;
use anyhow::{Result, ensure};

const RUNTIME_IDENTITY_MISMATCH_DETAIL: &str =
    "PTY fallback authority belongs to another runtime identity";
const RECOVERY_IDENTITY_MISMATCH_DETAIL: &str =
    "PTY fallback authority belongs to another recovery identity";
#[derive(Default)]
pub(super) struct ConfirmedBootstrapFallbackState {
    recovery_identity_sha256: Option<Sha256Digest>,
    global_exclusion_committed: bool,
}

impl PtyMasterAliasVerifier {
    pub(in crate::process) fn bind_secondary_custody_recovery(
        &self,
        runtime: &CustodyRuntimeIdentity,
        recovery_identity_sha256: Sha256Digest,
    ) -> Result<()> {
        ensure!(
            runtime_matches(&self.identity, runtime),
            RUNTIME_IDENTITY_MISMATCH_DETAIL
        );
        let mut state = lock_fallback(&self.identity);
        if let Some(bound) = state.recovery_identity_sha256 {
            ensure!(
                bound == recovery_identity_sha256,
                RECOVERY_IDENTITY_MISMATCH_DETAIL
            );
        } else {
            state.recovery_identity_sha256 = Some(recovery_identity_sha256);
        }
        Ok(())
    }

    pub(in crate::process) fn commit_confirmed_bootstrap_global_exclusion(&self) {
        let mut state = lock_fallback(&self.identity);
        state.global_exclusion_committed = state.recovery_identity_sha256.is_some();
    }
}

impl PtyHostReleaseVerifier {
    pub fn release_control(&self, offer: &CustodyOffer) -> Result<CustodyControl> {
        ensure!(
            runtime_matches(&self.identity, &offer.runtime),
            RUNTIME_IDENTITY_MISMATCH_DETAIL
        );
        let state = lock_fallback(&self.identity);
        let Some(bound) = state.recovery_identity_sha256 else {
            return CustodyControl::release(offer);
        };
        ensure!(
            bound == offer.recovery_identity_sha256,
            RECOVERY_IDENTITY_MISMATCH_DETAIL
        );
        if !state.global_exclusion_committed
            || !matches!(
                offer.descriptor_set,
                crate::pty_custody::CustodyDescriptorSet::MacosProcessCapability { .. }
            )
        {
            return CustodyControl::release(offer);
        }
        let settlement = CustodyReleaseSettlement::confirmed_bootstrap_global_exclusion(offer)?;
        CustodyControl::release_with_settlement(offer, settlement)
    }
}

fn runtime_matches(identity: &PtyClosureIdentity, runtime: &CustodyRuntimeIdentity) -> bool {
    identity.runtime.terminal_id == runtime.terminal_id
        && identity.runtime.runtime_epoch == runtime.runtime_epoch
        && identity.root.pid == runtime.child_pid
        && identity.root.start_time == runtime.child_start_time
}

fn lock_fallback(
    identity: &PtyClosureIdentity,
) -> std::sync::MutexGuard<'_, ConfirmedBootstrapFallbackState> {
    identity
        .fallback
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
