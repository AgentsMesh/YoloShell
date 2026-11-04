#[cfg(not(target_os = "macos"))]
use super::CapabilityScope;
use super::PtySessionProof;
use anyhow::{Result, bail, ensure};

const REOPEN_SEAL_AUTHORITY_MISMATCH_DETAIL: &str =
    "PTY reopen-seal authority does not apply to this process proof";
#[cfg(target_os = "macos")]
const REOPEN_SEAL_MISSING_DETAIL: &str = "PTY reopen seal custody is unavailable";

pub(crate) struct PtyReopenSealAuthority(ReopenSealAuthority);

#[derive(Clone)]
pub struct CommittedPtyReopenSealAuthority(ReopenSealAuthority);

enum ReopenSealAuthority {
    #[cfg(target_os = "macos")]
    Installed(super::macos_reopen_seal::MacosPtyReopenSealIdentity),
    NotApplicable(NotApplicableReason),
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum NotApplicableReason {
    MissingProof,
    ProcessDiscovery,
    UnsupportedPlatform,
}

impl PtyReopenSealAuthority {
    pub(in crate::process) fn missing_proof() -> Self {
        Self(ReopenSealAuthority::NotApplicable(
            NotApplicableReason::MissingProof,
        ))
    }

    pub(in crate::process) fn validate_missing_proof(self) -> Result<()> {
        ensure!(
            matches!(
                self.0,
                ReopenSealAuthority::NotApplicable(NotApplicableReason::MissingProof)
            ),
            REOPEN_SEAL_AUTHORITY_MISMATCH_DETAIL
        );
        Ok(())
    }

    pub(in crate::process) fn committed_identity(&self) -> CommittedPtyReopenSealAuthority {
        CommittedPtyReopenSealAuthority(self.0.clone())
    }
}

impl CommittedPtyReopenSealAuthority {
    pub fn same_authority(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            #[cfg(target_os = "macos")]
            (ReopenSealAuthority::Installed(left), ReopenSealAuthority::Installed(right)) => {
                left.same_authority(right)
            }
            (
                ReopenSealAuthority::NotApplicable(left),
                ReopenSealAuthority::NotApplicable(right),
            ) => left == right,
            #[cfg(target_os = "macos")]
            (ReopenSealAuthority::Installed(_), ReopenSealAuthority::NotApplicable(_))
            | (ReopenSealAuthority::NotApplicable(_), ReopenSealAuthority::Installed(_)) => false,
        }
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn missing_proof_for_test() -> Self {
        PtyReopenSealAuthority::missing_proof().committed_identity()
    }
}

impl Clone for ReopenSealAuthority {
    fn clone(&self) -> Self {
        match self {
            #[cfg(target_os = "macos")]
            Self::Installed(identity) => Self::Installed(identity.clone()),
            Self::NotApplicable(reason) => Self::NotApplicable(*reason),
        }
    }
}

impl PtySessionProof {
    #[cfg(target_os = "macos")]
    fn with_reopen_seal<T>(
        &mut self,
        operation: impl FnOnce(
            &mut super::macos_reopen_seal::MacosPtyReopenSeal,
            &PtySessionProof,
        ) -> Result<T>,
    ) -> Result<Option<T>> {
        use super::macos_reopen_seal_state::MacosPtyReopenSealState;
        let mut seal = match std::mem::replace(&mut self.reopen_seal, MacosPtyReopenSealState::Idle)
        {
            MacosPtyReopenSealState::Active(seal) => seal,
            state => {
                self.reopen_seal = state;
                return Ok(None);
            }
        };
        let outcome =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| operation(&mut seal, self)));
        self.reopen_seal = MacosPtyReopenSealState::Active(seal);
        match outcome {
            Ok(result) => result.map(Some),
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    pub(in crate::process) fn acquire_reopen_seal(&mut self) -> Result<PtyReopenSealAuthority> {
        if self.is_process_discovery() {
            return Ok(PtyReopenSealAuthority(ReopenSealAuthority::NotApplicable(
                NotApplicableReason::ProcessDiscovery,
            )));
        }
        #[cfg(target_os = "macos")]
        {
            use super::macos_reopen_seal_state::MacosPtyReopenSealState;
            if self.reopen_seal.is_idle() {
                let seal = super::macos_reopen_seal::MacosPtyReopenSeal::prepare(self)?;
                self.reopen_seal = MacosPtyReopenSealState::Active(seal);
            }
            let identity = self
                .with_reopen_seal(|seal, proof| {
                    seal.activate()?;
                    seal.revalidate(proof)?;
                    Ok(seal.identity())
                })?
                .ok_or_else(|| anyhow::anyhow!(REOPEN_SEAL_MISSING_DETAIL))?;
            return Ok(PtyReopenSealAuthority(ReopenSealAuthority::Installed(
                identity,
            )));
        }
        #[cfg(not(target_os = "macos"))]
        Ok(PtyReopenSealAuthority(ReopenSealAuthority::NotApplicable(
            NotApplicableReason::UnsupportedPlatform,
        )))
    }

    pub(in crate::process) fn install_reopen_seal(
        &mut self,
        authority: PtyReopenSealAuthority,
    ) -> Result<()> {
        #[cfg(target_os = "macos")]
        match authority.0 {
            ReopenSealAuthority::Installed(identity) if !self.is_process_discovery() => {
                let installed = self.with_reopen_seal(|seal, proof| {
                    ensure!(
                        seal.has_identity(&identity),
                        REOPEN_SEAL_AUTHORITY_MISMATCH_DETAIL
                    );
                    seal.revalidate(proof)
                })?;
                ensure!(installed.is_some(), REOPEN_SEAL_AUTHORITY_MISMATCH_DETAIL);
                return Ok(());
            }
            ReopenSealAuthority::NotApplicable(NotApplicableReason::ProcessDiscovery)
                if self.is_process_discovery() =>
            {
                return Ok(());
            }
            _ => bail!(REOPEN_SEAL_AUTHORITY_MISMATCH_DETAIL),
        }
        #[cfg(not(target_os = "macos"))]
        match authority.0 {
            ReopenSealAuthority::NotApplicable(NotApplicableReason::ProcessDiscovery)
                if self.is_process_discovery() =>
            {
                Ok(())
            }
            ReopenSealAuthority::NotApplicable(NotApplicableReason::UnsupportedPlatform)
                if self.scope == CapabilityScope::Pty =>
            {
                Ok(())
            }
            _ => bail!(REOPEN_SEAL_AUTHORITY_MISMATCH_DETAIL),
        }
    }
}

#[path = "pty_session_reopen_seal_committed.rs"]
mod committed;

#[cfg(target_os = "macos")]
include!("pty_session_reopen_seal_lifecycle_macos.rs");
#[cfg(not(target_os = "macos"))]
include!("pty_session_reopen_seal_lifecycle_other.rs");

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/reopen/pty_session_reopen_seal_tests.rs"]
mod tests;
