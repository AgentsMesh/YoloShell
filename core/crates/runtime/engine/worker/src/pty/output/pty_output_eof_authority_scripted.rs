use super::*;
use crate::process::kernel_eof_authority::bind_scripted_kernel_eof;

impl KernelPtyEofAuthority {
    pub(crate) fn identity_for_test(&self) -> Arc<KernelPtyEofIdentity> {
        Arc::clone(self.source.identity())
    }

    pub(crate) fn scripted_reader(
        version: &ProcessClosureVersion,
        lifecycle: &Arc<ReaderLifecycle>,
    ) -> Option<Self> {
        let identity = lifecycle.kernel_eof_identity()?;
        let (permit, validation) = bind_scripted_kernel_eof(version, &identity);
        Some(Self {
            permit: Some(permit),
            validation: Some(validation),
            source: KernelPtyEofSource::Reader {
                lifecycle: Arc::clone(lifecycle),
                identity,
            },
            seal_epoch: None,
            output_invalidated: std::cell::Cell::new(false),
        })
    }

    pub(crate) fn validates_scripted_reader(
        &self,
        version: &ProcessClosureVersion,
        lifecycle: &Arc<ReaderLifecycle>,
    ) -> bool {
        let Some(validation) = self.validation_authority() else {
            return false;
        };
        match &self.source {
            KernelPtyEofSource::Reader {
                lifecycle: expected,
                identity,
            } if Arc::ptr_eq(expected, lifecycle) => {
                validation.matches_scripted(version, identity)
                    && lifecycle.validates_kernel_eof_identity(identity)
            }
            _ => false,
        }
    }

    pub(crate) fn validate_scripted_reader(
        &self,
        version: &ProcessClosureVersion,
        lifecycle: &Arc<ReaderLifecycle>,
    ) -> Option<ValidatedKernelEof> {
        if !self.validates_scripted_reader(version, lifecycle) {
            return None;
        }
        let identity = match &self.source {
            KernelPtyEofSource::Reader { identity, .. } => Arc::clone(identity),
            KernelPtyEofSource::ReaderProbe { .. } | KernelPtyEofSource::Exclusive(_) => {
                return None;
            }
        };
        self.validation_authority()?
            .validate_scripted(version, &identity)
    }
}
