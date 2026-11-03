use super::binding_transition::{PendingBindingTransition, TRANSITION_TRANSCRIPT_CHANGED};
use crate::paths::identity::ObjectIdentity;
use anyhow::{Result, ensure};
use std::path::Path;

impl PendingBindingTransition {
    pub(super) fn verify_transcript(&self) -> Result<()> {
        let identity = transcript_identity(
            Path::new(&self.predecessor.binding.data_home),
            &self.predecessor.transcript_path,
        )?;
        ensure!(
            identity == Some(self.predecessor.transcript_identity),
            TRANSITION_TRANSCRIPT_CHANGED
        );
        Ok(())
    }

    pub(super) fn validate_shape(&self) -> Result<()> {
        super::binding_transition_validation::validate_shape(self)
    }
}

fn transcript_identity(data_home: &Path, path: &Path) -> Result<Option<ObjectIdentity>> {
    super::super::authoritative_evidence::named_transcript_identity(data_home, path)
}
