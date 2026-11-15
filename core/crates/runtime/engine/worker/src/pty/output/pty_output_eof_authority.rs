use crate::process::kernel_eof_authority::{
    KernelEofPermit, KernelEofValidationAuthority, ValidatedKernelEof, bind_kernel_eof,
};
use crate::process::{ProcessClosureVersion, PtyMasterAliasReleaseAuthority};
use crate::worker::ReaderLifecycle;
use std::sync::Arc;

use super::CommittedKernelEofSealEpoch;

pub(crate) use crate::process::kernel_eof_authority::KernelPtyEofIdentity;

pub(crate) struct KernelPtyEofAuthority {
    pub(crate) permit: Option<KernelEofPermit>,
    pub(crate) validation: Option<KernelEofValidationAuthority>,
    pub(crate) source: KernelPtyEofSource,
    pub(crate) seal_epoch: Option<CommittedKernelEofSealEpoch>,
    pub(crate) output_invalidated: std::cell::Cell<bool>,
}

pub(crate) enum KernelPtyEofSource {
    Reader {
        lifecycle: Arc<ReaderLifecycle>,
        identity: Arc<KernelPtyEofIdentity>,
    },
    ReaderProbe {
        lifecycle: Arc<ReaderLifecycle>,
        identity: Arc<KernelPtyEofIdentity>,
    },
    Exclusive(Arc<KernelPtyEofIdentity>),
}

#[path = "pty_output_eof_authority_exclusive.rs"]
mod exclusive;
#[path = "pty_output_eof_authority_reader_probe.rs"]
mod reader_probe;
#[cfg(test)]
#[path = "pty_output_eof_authority_scripted.rs"]
mod scripted;
#[path = "pty_output_eof_authority_sealed.rs"]
mod sealed;

#[derive(Clone, Copy)]
pub(crate) enum KernelEofSourceKind {
    Reader,
    ReaderProbe,
}

impl KernelPtyEofAuthority {
    pub(crate) fn reader(
        version: &ProcessClosureVersion,
        binding: &Arc<PtyMasterAliasReleaseAuthority>,
        lifecycle: &Arc<ReaderLifecycle>,
    ) -> Option<Self> {
        let (identity, seal) = lifecycle.kernel_eof_observation()?;
        let (permit, validation) = bind_kernel_eof(version, binding, &identity)?;
        let seal_epoch =
            seal.map(|seal| CommittedKernelEofSealEpoch::new(Arc::clone(&identity), seal));
        Some(Self {
            permit: Some(permit),
            validation: Some(validation),
            source: KernelPtyEofSource::Reader {
                lifecycle: Arc::clone(lifecycle),
                identity,
            },
            seal_epoch,
            output_invalidated: std::cell::Cell::new(false),
        })
    }

    pub(crate) fn take_permit(&mut self) -> Option<KernelEofPermit> {
        if self.output_invalidated.get() {
            return None;
        }
        self.permit.take()
    }

    pub(crate) fn invalidate_output(&self) {
        self.output_invalidated.set(true);
    }

    pub(crate) fn output_is_invalidated(&self) -> bool {
        self.output_invalidated.get()
    }

    pub(crate) fn validation_authority(&self) -> Option<&KernelEofValidationAuthority> {
        (!self.output_invalidated.get())
            .then(|| self.validation.as_ref())
            .flatten()
    }

    pub(crate) fn identity_for_source<'authority>(
        &'authority self,
        source: KernelEofSourceKind,
        version: &ProcessClosureVersion,
        binding: &Arc<PtyMasterAliasReleaseAuthority>,
        lifecycle: &Arc<ReaderLifecycle>,
    ) -> Option<&'authority Arc<KernelPtyEofIdentity>> {
        let validation = self.validation_authority()?;
        let (expected, identity) = match (&self.source, source) {
            (
                KernelPtyEofSource::Reader {
                    lifecycle: expected,
                    identity,
                },
                KernelEofSourceKind::Reader,
            )
            | (
                KernelPtyEofSource::ReaderProbe {
                    lifecycle: expected,
                    identity,
                },
                KernelEofSourceKind::ReaderProbe,
            ) => (expected, identity),
            _ => return None,
        };
        (Arc::ptr_eq(expected, lifecycle) && validation.matches_current(version, binding, identity))
            .then_some(identity)
    }

    pub(crate) fn validate_source(
        &self,
        source: KernelEofSourceKind,
        version: &ProcessClosureVersion,
        binding: &Arc<PtyMasterAliasReleaseAuthority>,
        lifecycle: &Arc<ReaderLifecycle>,
    ) -> Option<ValidatedKernelEof> {
        let identity = Arc::clone(self.identity_for_source(source, version, binding, lifecycle)?);
        self.validation_authority()?
            .validate_current(version, binding, &identity)
    }

    pub(crate) fn reader_identity<'authority>(
        &'authority self,
        version: &ProcessClosureVersion,
        binding: &Arc<PtyMasterAliasReleaseAuthority>,
        lifecycle: &Arc<ReaderLifecycle>,
    ) -> Option<&'authority Arc<KernelPtyEofIdentity>> {
        self.identity_for_source(KernelEofSourceKind::Reader, version, binding, lifecycle)
    }

    pub(crate) fn validate_reader(
        &self,
        version: &ProcessClosureVersion,
        binding: &Arc<PtyMasterAliasReleaseAuthority>,
        lifecycle: &Arc<ReaderLifecycle>,
    ) -> Option<ValidatedKernelEof> {
        self.validate_source(KernelEofSourceKind::Reader, version, binding, lifecycle)
    }

    pub(crate) fn retire_validation(&mut self) {
        self.validation = None;
    }

    pub(crate) fn committed_seal_epoch(&self) -> Option<CommittedKernelEofSealEpoch> {
        self.seal_epoch.clone()
    }
}

impl KernelPtyEofSource {
    pub(crate) fn identity(&self) -> &Arc<KernelPtyEofIdentity> {
        match self {
            Self::Reader { identity, .. }
            | Self::ReaderProbe { identity, .. }
            | Self::Exclusive(identity) => identity,
        }
    }
}
