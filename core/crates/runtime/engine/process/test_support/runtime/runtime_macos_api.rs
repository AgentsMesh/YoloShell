use crate::{
    LiveProcessIdentity, MacosProcessCapabilityChildWriter, MacosProcessCapabilityManagerReader,
    MacosProcessCapabilityRelease,
};
use anyhow::Result;
use std::process::Command;
use std::sync::Arc;

pub use yolo_process_identity::macos_kernel::KernelProcessMetadata as KernelProcessMetadataForTest;

pub fn kernel_process_metadata_for_test(pid: u32) -> Result<KernelProcessMetadataForTest> {
    crate::macos_native::kernel_process_metadata(pid)
}

pub fn with_test_signal_capability<T>(available: bool, operation: impl FnOnce() -> T) -> T {
    crate::macos_signal::with_test_capability(available, operation)
}

impl MacosProcessCapabilityChildWriter {
    pub fn configure_exec_inheritance_for_test(&self, command: &mut Command) {
        self.configure_exec_inheritance(command);
    }
}

impl MacosProcessCapabilityManagerReader {
    pub fn configure_exec_inheritance_for_test(&self, command: &mut Command) {
        self.configure_exec_inheritance(command);
    }

    pub fn verify_writer_holder_for_test(&self, child: &LiveProcessIdentity) -> Result<()> {
        self.verify_writer_holder(child)
    }

    pub fn writer_handle_for_test(&self) -> u64 {
        self.writer_handle()
    }

    pub fn poll_release_for_test(&self) -> Result<MacosProcessCapabilityRelease> {
        self.poll_release()
    }

    pub fn confirm_root_exit_for_test(&self, root: &LiveProcessIdentity) -> Result<()> {
        self.confirm_root_exit(root)
    }
}

impl crate::MacosProcessCapabilityIssuer {
    pub fn into_staged_for_test(
        self,
    ) -> Result<(
        Arc<MacosProcessCapabilityManagerReader>,
        MacosProcessCapabilityChildWriter,
    )> {
        self.into_staged()
    }
}

impl crate::MacosSecondaryCustodyExport {
    pub fn capture_for_test(
        proof: &crate::PtySessionProof,
        runtime: &crate::pty_custody::CustodyRuntimeIdentity,
    ) -> Result<Self> {
        Self::capture(proof, runtime)
    }
}
