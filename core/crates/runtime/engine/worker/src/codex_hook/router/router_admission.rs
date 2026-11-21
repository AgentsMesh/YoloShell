use super::error::CodexRouterError;
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexAdmissionPublication, CodexAdmissionRecord, CodexAdmissionStorage,
};
use std::path::Path;

#[derive(Clone, Debug)]
pub(in crate::worker::codex_hook) struct DurableCodexAdmission {
    authority: CodexAdmissionAuthority,
}

impl DurableCodexAdmission {
    pub(in crate::worker::codex_hook) fn from_authority(
        authority: CodexAdmissionAuthority,
    ) -> Result<Self, CodexRouterError> {
        authority
            .verify()
            .map_err(|_| CodexRouterError::Admission)?;
        Ok(Self { authority })
    }

    pub(in crate::worker::codex_hook) fn from_publication(
        storage: CodexAdmissionStorage,
        publication: CodexAdmissionPublication,
    ) -> Result<Self, CodexRouterError> {
        CodexAdmissionAuthority::from_publication(storage, publication)
            .map(|authority| Self { authority })
            .map_err(|_| CodexRouterError::Admission)
    }

    pub(in crate::worker::codex_hook) fn recover(
        storage: CodexAdmissionStorage,
    ) -> Result<Option<Self>, CodexRouterError> {
        CodexAdmissionAuthority::recover(storage)
            .map(|authority| authority.map(|authority| Self { authority }))
            .map_err(|_| CodexRouterError::Admission)
    }

    pub(in crate::worker::codex_hook) fn verify(&self) -> Result<(), CodexRouterError> {
        self.authority
            .verify()
            .map_err(|_| CodexRouterError::Admission)
    }

    pub(in crate::worker::codex_hook) fn record(&self) -> &CodexAdmissionRecord {
        self.authority.record()
    }

    pub(in crate::worker::codex_hook) fn path(&self) -> &Path {
        self.authority.path()
    }

    pub(super) fn exactly_matches(&self, other: &Self) -> bool {
        self.authority.exactly_matches(&other.authority)
    }

    pub(in crate::worker::codex_hook) fn authority(&self) -> &CodexAdmissionAuthority {
        &self.authority
    }
}
