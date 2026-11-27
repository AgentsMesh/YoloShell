use super::retirement_constants::{CORRUPT, STORAGE};
use super::retirement_storage::CodexRetirementStorage;
use super::{
    CodexAdmissionAuthority, CodexInvocationAuthority, CodexRetirementSeal,
    CodexRetirementSealArtifact, CodexRetirementSealPublication,
};
use anyhow::{Context, Result, ensure};

#[path = "retirement_state.rs"]
mod state;
pub use state::{CodexRetiredState, CodexRetirementState};
#[path = "retirement_writer_acquire.rs"]
mod acquire;
#[path = "retirement_writer_terminal.rs"]
mod terminal;

pub struct CodexRetirementWriterFence {
    pub(super) storage: CodexRetirementStorage,
    lock: crate::paths::private_file_lock::PrivateFileLock,
}

impl CodexRetirementWriterFence {
    pub fn verify_unsealed(&self) -> Result<()> {
        self.verify_lock()?;
        self.storage.ensure_unsealed()?;
        self.verify_lock()
    }

    pub fn publish_seal(
        &self,
        seal: &CodexRetirementSeal,
        admission: &CodexAdmissionAuthority,
    ) -> Result<CodexRetirementSealPublication> {
        self.verify_seal_candidate(seal, admission)?;
        let publication = self
            .storage
            .publish_seal(seal, || self.verify_seal_candidate(seal, admission))?;
        self.verify_seal(publication.artifact(), admission)?;
        Ok(publication)
    }

    pub fn verify_seal(
        &self,
        seal: &CodexRetirementSealArtifact,
        admission: &CodexAdmissionAuthority,
    ) -> Result<()> {
        self.verify_seal_candidate(seal.record(), admission)?;
        self.storage.verify_seal(seal)?;
        self.verify_seal_candidate(seal.record(), admission)
    }

    fn verify_seal_candidate(
        &self,
        seal: &CodexRetirementSeal,
        admission: &CodexAdmissionAuthority,
    ) -> Result<()> {
        let authority = self.invocation_authority(admission)?;
        seal.validate_for(admission)?;
        self.verify_invocation_authority(authority, admission)
    }

    pub(super) fn invocation_authority(
        &self,
        admission: &CodexAdmissionAuthority,
    ) -> Result<CodexInvocationAuthority> {
        let authority = self.storage.invocation_authority()?;
        self.verify_invocation_authority(authority, admission)?;
        Ok(authority)
    }

    fn verify_invocation_authority(
        &self,
        expected: CodexInvocationAuthority,
        admission: &CodexAdmissionAuthority,
    ) -> Result<()> {
        self.verify_lock()?;
        admission.verify().context(STORAGE)?;
        ensure!(self.storage.invocation_authority()? == expected, CORRUPT);
        ensure!(admission.invocation_authority()? == expected, CORRUPT);
        admission.verify().context(STORAGE)?;
        self.verify_lock()
    }

    fn verify_lock(&self) -> Result<()> {
        self.storage.directory().verify_path().context(STORAGE)?;
        self.lock.verify().context(STORAGE)?;
        self.storage.directory().verify_path().context(STORAGE)
    }
}

#[cfg(test)]
thread_local! {
    static AFTER_ACQUIRE_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
impl CodexRetirementWriterFence {
    pub(crate) fn set_after_acquire_hook(hook: impl FnOnce() + 'static) {
        AFTER_ACQUIRE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
    }
}

#[cfg(test)]
fn run_after_acquire_hook() {
    AFTER_ACQUIRE_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(not(test))]
fn run_after_acquire_hook() {}
