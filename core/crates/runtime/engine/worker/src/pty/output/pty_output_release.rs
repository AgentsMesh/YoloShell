use super::{ExclusivePtyOutput, PtyOutputRecovery};
use crate::process::{PtyMasterAliasReleaseAuthority, PtyMasterAliasesClosed};
use crate::worker::pty_runtime_alias_custody::{
    PtyRuntimeAliasClosePoll, PtyRuntimeAliasReleasePoll, PtyRuntimeLocalAliasRelease,
};
use anyhow::{Context, Result};

pub(crate) const READER_NOT_DRAINED_DETAIL: &str =
    "PTY master aliases cannot close before the output reader is drained";
pub(crate) const OWNER_MASTER_NOT_RELEASED_DETAIL: &str =
    "PTY output descriptor cannot close before its startup master owner";
pub(crate) const MASTER_ALIAS_AUTHORITY_UNAVAILABLE_CONTEXT: &str =
    "PTY master-alias closure authority is unavailable";
#[cfg(target_os = "macos")]
pub(crate) const MASTER_ALIAS_OBJECT_MISMATCH_DETAIL: &str =
    "PTY output descriptor belongs to another PTY object";
pub(crate) const RUNTIME_CUSTODY_UNAVAILABLE_CONTEXT: &str =
    "runtime PTY output has no secondary-custody release authority";

pub(crate) enum MasterAliasReleasePoll {
    Pending,
    Closed(PtyMasterAliasesClosed),
    Delivered,
}

impl PtyOutputRecovery {
    pub fn release_master_aliases(&mut self) -> Result<MasterAliasReleasePoll> {
        if let Some(lifecycle) = self.reader_lifecycle.as_ref() {
            anyhow::ensure!(
                lifecycle.master_aliases_can_close(),
                READER_NOT_DRAINED_DETAIL
            );
        }
        let runtime = self
            .descriptor
            .runtime
            .clone()
            .context(RUNTIME_CUSTODY_UNAVAILABLE_CONTEXT)?;
        let descriptor = std::sync::Arc::clone(&self.descriptor);
        let authority = self.master_alias_authority()?;
        runtime
            .release(|| match runtime.close_local_aliases()? {
                PtyRuntimeAliasClosePoll::Pending => Ok(PtyRuntimeLocalAliasRelease::Pending),
                PtyRuntimeAliasClosePoll::Closed => {
                    descriptor.release();
                    authority
                        .take_local_aliases_closed()
                        .map(PtyRuntimeLocalAliasRelease::closed)
                }
            })
            .map(MasterAliasReleasePoll::from)
    }
}

impl ExclusivePtyOutput {
    pub(crate) fn mark_owner_master_released(&mut self) {
        self.owner_master_released = true;
    }

    pub fn release_master_aliases(&mut self) -> Result<MasterAliasReleasePoll> {
        let authority = self.master_alias_authority()?;
        anyhow::ensure!(self.owner_master_released, OWNER_MASTER_NOT_RELEASED_DETAIL);
        self.release_startup_master_aliases(&authority)
    }

    pub(crate) fn bind_master_alias_authority(
        &mut self,
        authority: PtyMasterAliasReleaseAuthority,
    ) -> Result<()> {
        anyhow::ensure!(
            self.master_alias_authority.is_none(),
            MASTER_ALIAS_AUTHORITY_UNAVAILABLE_CONTEXT
        );
        #[cfg(target_os = "macos")]
        anyhow::ensure!(
            match &self.descriptor {
                super::OutputDescriptor::Native(descriptor) => {
                    authority.validates_output_descriptor(descriptor)?
                }
                super::OutputDescriptor::Released => false,
                #[cfg(test)]
                super::OutputDescriptor::Observed(_) | super::OutputDescriptor::Quiescent(_) =>
                    true,
            },
            MASTER_ALIAS_OBJECT_MISMATCH_DETAIL
        );
        self.master_alias_authority = Some(std::sync::Arc::new(authority));
        Ok(())
    }

    pub(crate) fn master_alias_authority(
        &self,
    ) -> Result<std::sync::Arc<PtyMasterAliasReleaseAuthority>> {
        self.master_alias_authority
            .as_ref()
            .cloned()
            .context(MASTER_ALIAS_AUTHORITY_UNAVAILABLE_CONTEXT)
    }
}

impl PtyOutputRecovery {
    pub(crate) fn master_alias_authority(
        &self,
    ) -> Result<std::sync::Arc<PtyMasterAliasReleaseAuthority>> {
        self.master_alias_authority
            .as_ref()
            .cloned()
            .context(MASTER_ALIAS_AUTHORITY_UNAVAILABLE_CONTEXT)
    }
}

impl From<PtyRuntimeAliasReleasePoll> for MasterAliasReleasePoll {
    fn from(poll: PtyRuntimeAliasReleasePoll) -> Self {
        match poll {
            PtyRuntimeAliasReleasePoll::Pending => Self::Pending,
            PtyRuntimeAliasReleasePoll::Closed(proof) => Self::Closed(proof),
            PtyRuntimeAliasReleasePoll::Delivered => Self::Delivered,
        }
    }
}
