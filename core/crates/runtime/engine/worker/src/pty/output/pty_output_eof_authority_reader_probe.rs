use super::*;

impl KernelPtyEofAuthority {
    pub(in super::super) fn reader_probe(
        version: &ProcessClosureVersion,
        binding: &Arc<PtyMasterAliasReleaseAuthority>,
        lifecycle: &Arc<ReaderLifecycle>,
        identity: &Arc<KernelPtyEofIdentity>,
    ) -> Option<Self> {
        let (permit, validation) = bind_kernel_eof(version, binding, identity)?;
        Some(Self {
            permit: Some(permit),
            validation: Some(validation),
            source: KernelPtyEofSource::ReaderProbe {
                lifecycle: Arc::clone(lifecycle),
                identity: Arc::clone(identity),
            },
            seal_epoch: None,
            output_invalidated: std::cell::Cell::new(false),
        })
    }

    pub(in super::super) fn reader_probe_identity<'authority>(
        &'authority self,
        version: &ProcessClosureVersion,
        binding: &Arc<PtyMasterAliasReleaseAuthority>,
        lifecycle: &Arc<ReaderLifecycle>,
    ) -> Option<&'authority Arc<KernelPtyEofIdentity>> {
        self.identity_for_source(
            KernelEofSourceKind::ReaderProbe,
            version,
            binding,
            lifecycle,
        )
    }

    pub(in super::super) fn validate_reader_probe(
        &self,
        version: &ProcessClosureVersion,
        binding: &Arc<PtyMasterAliasReleaseAuthority>,
        lifecycle: &Arc<ReaderLifecycle>,
    ) -> Option<ValidatedKernelEof> {
        self.validate_source(
            KernelEofSourceKind::ReaderProbe,
            version,
            binding,
            lifecycle,
        )
    }
}
