use super::{ProcessClosureVersion, PtyMasterAliasReleaseAuthority};
use std::sync::Arc;

#[derive(Debug)]
pub struct KernelPtyEofIdentity {
    _private: (),
}

struct KernelEofBinding {
    version: ProcessClosureVersion,
    output: Arc<KernelPtyEofIdentity>,
    master_alias: KernelEofMasterAliasBinding,
}

enum KernelEofMasterAliasBinding {
    Authority(Arc<PtyMasterAliasReleaseAuthority>),
    #[cfg(any(test, feature = "runtime-component-tests"))]
    Scripted,
}

pub struct KernelEofPermit(Arc<KernelEofBinding>);
pub struct KernelEofValidationAuthority(Arc<KernelEofBinding>);
pub(crate) struct KernelEofCommitBinding(Arc<KernelEofBinding>);
pub struct ValidatedKernelEof(Arc<KernelEofBinding>);

#[path = "kernel_eof_authority_sealed.rs"]
mod sealed;
pub use sealed::ValidatedSealedKernelEof;

impl KernelPtyEofIdentity {
    pub fn from_kernel_read() -> Arc<Self> {
        Arc::new(Self { _private: () })
    }
}

pub fn bind_kernel_eof(
    version: &ProcessClosureVersion,
    master_alias: &Arc<PtyMasterAliasReleaseAuthority>,
    output: &Arc<KernelPtyEofIdentity>,
) -> Option<(KernelEofPermit, KernelEofValidationAuthority)> {
    version.binds_pty_output(master_alias).then(|| {
        paired(KernelEofBinding {
            version: version.clone(),
            output: Arc::clone(output),
            master_alias: KernelEofMasterAliasBinding::Authority(Arc::clone(master_alias)),
        })
    })
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub fn bind_scripted_kernel_eof(
    version: &ProcessClosureVersion,
    output: &Arc<KernelPtyEofIdentity>,
) -> (KernelEofPermit, KernelEofValidationAuthority) {
    paired(KernelEofBinding {
        version: version.clone(),
        output: Arc::clone(output),
        master_alias: KernelEofMasterAliasBinding::Scripted,
    })
}

fn paired(binding: KernelEofBinding) -> (KernelEofPermit, KernelEofValidationAuthority) {
    let binding = Arc::new(binding);
    (
        KernelEofPermit(Arc::clone(&binding)),
        KernelEofValidationAuthority(binding),
    )
}

impl KernelEofPermit {
    pub(crate) fn authorize(
        self,
        version: &ProcessClosureVersion,
    ) -> Option<KernelEofCommitBinding> {
        (&self.0.version == version).then(|| KernelEofCommitBinding(self.0))
    }
}

impl KernelEofValidationAuthority {
    pub fn matches_current(
        &self,
        version: &ProcessClosureVersion,
        master_alias: &Arc<PtyMasterAliasReleaseAuthority>,
        output: &Arc<KernelPtyEofIdentity>,
    ) -> bool {
        self.0.matches_current(version, master_alias, output)
    }

    pub fn validate_current(
        &self,
        version: &ProcessClosureVersion,
        master_alias: &Arc<PtyMasterAliasReleaseAuthority>,
        output: &Arc<KernelPtyEofIdentity>,
    ) -> Option<ValidatedKernelEof> {
        self.0
            .matches_current(version, master_alias, output)
            .then(|| ValidatedKernelEof(Arc::clone(&self.0)))
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn matches_scripted(
        &self,
        version: &ProcessClosureVersion,
        output: &Arc<KernelPtyEofIdentity>,
    ) -> bool {
        self.0.matches_scripted(version, output)
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn validate_scripted(
        &self,
        version: &ProcessClosureVersion,
        output: &Arc<KernelPtyEofIdentity>,
    ) -> Option<ValidatedKernelEof> {
        self.0
            .matches_scripted(version, output)
            .then(|| ValidatedKernelEof(Arc::clone(&self.0)))
    }
}

impl KernelEofCommitBinding {
    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn validated_for_test(&self) -> ValidatedKernelEof {
        ValidatedKernelEof(Arc::clone(&self.0))
    }
}

impl KernelEofBinding {
    fn matches_current(
        &self,
        version: &ProcessClosureVersion,
        master_alias: &Arc<PtyMasterAliasReleaseAuthority>,
        output: &Arc<KernelPtyEofIdentity>,
    ) -> bool {
        &self.version == version
            && Arc::ptr_eq(&self.output, output)
            && matches!(
                &self.master_alias,
                KernelEofMasterAliasBinding::Authority(expected)
                    if Arc::ptr_eq(expected, master_alias)
            )
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    fn matches_scripted(
        &self,
        version: &ProcessClosureVersion,
        output: &Arc<KernelPtyEofIdentity>,
    ) -> bool {
        &self.version == version
            && Arc::ptr_eq(&self.output, output)
            && matches!(&self.master_alias, KernelEofMasterAliasBinding::Scripted)
    }

    fn same_authority(&self, other: &Self) -> bool {
        self.version == other.version
            && Arc::ptr_eq(&self.output, &other.output)
            && self.master_alias.same_authority(&other.master_alias)
    }
}

impl KernelEofMasterAliasBinding {
    fn same_authority(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Authority(left), Self::Authority(right)) => Arc::ptr_eq(left, right),
            #[cfg(any(test, feature = "runtime-component-tests"))]
            (Self::Scripted, Self::Scripted) => true,
            #[cfg(any(test, feature = "runtime-component-tests"))]
            (Self::Authority(_), Self::Scripted) | (Self::Scripted, Self::Authority(_)) => false,
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/kernel_eof_authority_tests.rs"]
mod tests;
