use super::*;

impl KernelPtyEofAuthority {
    pub(crate) fn exclusive(
        version: &ProcessClosureVersion,
        binding: &Arc<PtyMasterAliasReleaseAuthority>,
        identity: &Arc<KernelPtyEofIdentity>,
        seal_epoch: Option<&CommittedKernelEofSealEpoch>,
    ) -> Option<Self> {
        if seal_epoch.is_some_and(|epoch| !epoch.is_current(identity)) {
            return None;
        }
        let (permit, validation) = bind_kernel_eof(version, binding, identity)?;
        Some(Self {
            permit: Some(permit),
            validation: Some(validation),
            source: KernelPtyEofSource::Exclusive(Arc::clone(identity)),
            seal_epoch: seal_epoch.cloned(),
            output_invalidated: std::cell::Cell::new(false),
        })
    }

    pub(crate) fn validates_exclusive(
        &self,
        version: &ProcessClosureVersion,
        binding: &Arc<PtyMasterAliasReleaseAuthority>,
        identity: Option<&Arc<KernelPtyEofIdentity>>,
        seal_epoch: Option<&CommittedKernelEofSealEpoch>,
    ) -> bool {
        let Some(validation) = self.validation_authority() else {
            return false;
        };
        match (&self.source, identity) {
            (KernelPtyEofSource::Exclusive(expected), Some(current)) => {
                Arc::ptr_eq(expected, current)
                    && validation.matches_current(version, binding, current)
                    && matching_seal_epochs(self.seal_epoch.as_ref(), seal_epoch)
            }
            _ => false,
        }
    }

    pub(crate) fn validate_exclusive(
        &self,
        version: &ProcessClosureVersion,
        binding: &Arc<PtyMasterAliasReleaseAuthority>,
        identity: Option<&Arc<KernelPtyEofIdentity>>,
        seal_epoch: Option<&CommittedKernelEofSealEpoch>,
    ) -> Option<ValidatedKernelEof> {
        let identity = Arc::clone(identity?);
        if !self.validates_exclusive(version, binding, Some(&identity), seal_epoch) {
            return None;
        }
        self.validation_authority()?
            .validate_current(version, binding, &identity)
    }
}

pub(crate) fn matching_seal_epochs(
    expected: Option<&CommittedKernelEofSealEpoch>,
    current: Option<&CommittedKernelEofSealEpoch>,
) -> bool {
    match (expected, current) {
        (Some(expected), Some(current)) => expected.same_authority(current),
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => false,
    }
}
