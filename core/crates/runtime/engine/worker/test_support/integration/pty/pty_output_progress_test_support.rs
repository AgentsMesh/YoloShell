use super::{ExclusivePtyOutput, OutputDescriptor, ReaderBackedPtyOutput};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize};

const STARTUP_CUSTODY_TEST_DEVICE: &str = "/dev/null";
const STARTUP_CUSTODY_TEST_CAPABILITY: &str = "startup-output-release-barrier";
const STARTUP_CUSTODY_TEST_PID: u32 = 44_401;
const STARTUP_CUSTODY_TEST_START: u64 = 81_401;
const EOF_BINDING_TEST_CAPABILITY: &str = "kernel-eof-output-binding";
const EOF_BINDING_TEST_PID: u32 = 44_402;
const EOF_BINDING_TEST_START: u64 = 81_402;

impl ExclusivePtyOutput {
    pub(in crate::worker) fn bind_kernel_eof_for_test(
        &mut self,
        master: &dyn portable_pty::MasterPty,
    ) -> crate::process::ProcessClosureVersion {
        let (authority, version) = Self::kernel_eof_binding_for_test(master);
        self.bind_master_alias_authority(authority).unwrap();
        version
    }

    pub(in crate::worker) fn kernel_eof_binding_for_test(
        master: &dyn portable_pty::MasterPty,
    ) -> (
        crate::process::PtyMasterAliasReleaseAuthority,
        crate::process::ProcessClosureVersion,
    ) {
        let mut proof = crate::process::PtySessionProof::capture_bound_for_test(
            master,
            EOF_BINDING_TEST_CAPABILITY.to_owned(),
        )
        .unwrap();
        proof
            .bind_runtime(uuid::Uuid::new_v4(), uuid::Uuid::new_v4())
            .unwrap();
        let authority = proof
            .bind_master_alias_release(crate::process::ProcessIdentity {
                pid: EOF_BINDING_TEST_PID,
                start_time: EOF_BINDING_TEST_START,
            })
            .unwrap();
        let version = crate::process::ProcessClosureVersion::for_pty_output_test(&authority);
        (authority, version)
    }

    pub(in crate::worker) fn observed(observed: Arc<AtomicUsize>) -> Self {
        Self {
            descriptor: OutputDescriptor::Observed(observed),
            startup_journal: None,
            kernel_eof: None,
            kernel_eof_seal_epoch: None,
            kernel_eof_committed: None,
            owner_master_released: false,
            output_abandoned: false,
            master_alias_authority: None,
            startup_custody: super::StartupCustodyReleaseState::NoSecondaryCustody,
        }
    }

    pub(in crate::worker) fn quiescent_for_test(observed: Arc<AtomicUsize>) -> Self {
        Self {
            descriptor: OutputDescriptor::Quiescent(observed),
            startup_journal: None,
            kernel_eof: None,
            kernel_eof_seal_epoch: None,
            kernel_eof_committed: None,
            owner_master_released: false,
            output_abandoned: false,
            master_alias_authority: None,
            startup_custody: super::StartupCustodyReleaseState::NoSecondaryCustody,
        }
    }

    pub(in crate::worker) fn make_quiescent_for_test(&mut self, observed: Arc<AtomicUsize>) {
        self.descriptor = OutputDescriptor::Quiescent(observed);
        self.startup_journal = None;
        self.kernel_eof = None;
        self.kernel_eof_seal_epoch = None;
        self.kernel_eof_committed = None;
    }

    pub(in crate::worker) fn kernel_eof_identity_for_test(
        &self,
    ) -> Option<Arc<super::KernelPtyEofIdentity>> {
        self.kernel_eof_seal_epoch
            .as_ref()
            .map(super::CommittedKernelEofSealEpoch::identity)
            .or(self.kernel_eof.as_ref())
            .cloned()
    }

    pub(in crate::worker) fn startup_custody_for_test() -> Self {
        let mut proof = crate::process::PtySessionProof::capture(
            STARTUP_CUSTODY_TEST_DEVICE.into(),
            STARTUP_CUSTODY_TEST_CAPABILITY.to_owned(),
        )
        .unwrap();
        proof
            .bind_runtime(uuid::Uuid::new_v4(), uuid::Uuid::new_v4())
            .unwrap();
        let authority = proof
            .bind_master_alias_release(crate::process::ProcessIdentity {
                pid: STARTUP_CUSTODY_TEST_PID,
                start_time: STARTUP_CUSTODY_TEST_START,
            })
            .unwrap();
        let mut output = Self::observed(Arc::new(AtomicUsize::new(usize::default())));
        output.bind_master_alias_authority(authority).unwrap();
        output
    }
}

impl ReaderBackedPtyOutput {
    pub(in crate::worker) fn observed(observed: Arc<AtomicUsize>) -> Self {
        Self::observed_with_runtime(observed, None, None)
    }

    pub(in crate::worker) fn observed_with_lifecycle(
        observed: Arc<AtomicUsize>,
        lifecycle: Arc<super::ReaderLifecycle>,
    ) -> Self {
        Self::observed_with_runtime(observed, Some(lifecycle), None)
    }

    pub(in crate::worker) fn observed_for_runtime(
        observed: Arc<AtomicUsize>,
        lifecycle: Arc<super::ReaderLifecycle>,
        child_waited: Arc<AtomicBool>,
    ) -> Self {
        Self::observed_with_runtime(observed, Some(lifecycle), Some(child_waited))
    }

    pub(in crate::worker) fn for_test() -> Self {
        Self::observed(Arc::new(AtomicUsize::new(usize::default())))
    }

    pub(in crate::worker) fn for_release_runtime_test(
        master: &dyn portable_pty::MasterPty,
        lifecycle: Arc<super::ReaderLifecycle>,
        child_waited: Arc<AtomicBool>,
        runtime: Arc<crate::worker::pty_runtime_alias_custody::PtyRuntimeAliasCustody>,
        authority: crate::process::PtyMasterAliasReleaseAuthority,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            descriptor: Arc::new(super::SharedOutputDescriptor::new(
                crate::worker::pty_output_terminal::capture(master)?,
                Some(runtime),
            )),
            reader_lifecycle: Some(lifecycle),
            child_waited: Some(child_waited),
            master_alias_authority: Some(Arc::new(authority)),
        })
    }

    pub(in crate::worker) fn observed_with_release_custody(
        observed: Arc<AtomicUsize>,
        lifecycle: Arc<super::ReaderLifecycle>,
        runtime: Arc<crate::worker::pty_runtime_alias_custody::PtyRuntimeAliasCustody>,
        authority: crate::process::PtyMasterAliasReleaseAuthority,
    ) -> Self {
        Self {
            descriptor: Arc::new(super::SharedOutputDescriptor::new(
                OutputDescriptor::Observed(observed),
                Some(runtime),
            )),
            reader_lifecycle: Some(lifecycle),
            child_waited: None,
            master_alias_authority: Some(Arc::new(authority)),
        }
    }

    fn observed_with_runtime(
        observed: Arc<AtomicUsize>,
        reader_lifecycle: Option<Arc<super::ReaderLifecycle>>,
        child_waited: Option<Arc<AtomicBool>>,
    ) -> Self {
        Self {
            descriptor: Arc::new(super::SharedOutputDescriptor::new(
                OutputDescriptor::Observed(observed),
                None,
            )),
            reader_lifecycle,
            child_waited,
            master_alias_authority: None,
        }
    }
}
