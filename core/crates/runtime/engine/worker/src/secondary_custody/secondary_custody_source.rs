use crate::pty_custody::{CustodyDescriptorSet, CustodyHolderIdentity, CustodyUnixStream};
use crate::sha256_digest::Sha256Digest;
use crate::worker::startup::startup_custody_transfer::StartupCustodyTransferFence;
use anyhow::Result;
use std::sync::{Arc, Mutex};

use super::secondary_custody_export::SecondaryCustodyExportState;

#[cfg(test)]
const INJECTED_INITIAL_HANDSHAKE_FAILURE_DETAIL: &str =
    "injected initial PTY custody handshake failure";
const CUSTODY_PLATFORM_UNAVAILABLE_DETAIL: &str =
    "PTY secondary-custody descriptor platform is unavailable";

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum InitialCustodyHandshakeBoundary {
    DescriptorSend,
    PreparedRead,
    DurablePersist,
    DurableInstalled,
    CommitWrite,
}

#[cfg(target_os = "linux")]
pub(crate) type PlatformCustodyExport = crate::process::LinuxSecondaryCustodyExport;
#[cfg(target_os = "macos")]
pub(crate) type PlatformCustodyExport = crate::process::MacosSecondaryCustodyExport;

pub(crate) struct WorkerSecondaryCustodySource {
    platform: Mutex<Option<Arc<PlatformCustodyExport>>>,
    pub(crate) successor_exports: SecondaryCustodyExportState,
    startup_transfer: Mutex<Option<Arc<StartupCustodyTransferFence>>>,
    #[cfg(test)]
    initial_fault: Option<InitialCustodyHandshakeBoundary>,
}

impl WorkerSecondaryCustodySource {
    pub(crate) fn new(
        platform: PlatformCustodyExport,
        startup_transfer: Arc<StartupCustodyTransferFence>,
    ) -> Self {
        Self {
            platform: Mutex::new(Some(Arc::new(platform))),
            successor_exports: SecondaryCustodyExportState::default(),
            startup_transfer: Mutex::new(Some(startup_transfer)),
            #[cfg(test)]
            initial_fault: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_faulted_for_test(
        platform: PlatformCustodyExport,
        startup_transfer: Arc<StartupCustodyTransferFence>,
        fault: InitialCustodyHandshakeBoundary,
    ) -> Self {
        let mut source = Self::new(platform, startup_transfer);
        source.initial_fault = Some(fault);
        source
    }

    pub(crate) fn descriptor_set(&self) -> CustodyDescriptorSet {
        self.platform()
            .expect(CUSTODY_PLATFORM_UNAVAILABLE_DETAIL)
            .descriptor_set()
    }

    pub(crate) fn recovery_identity_sha256(&self) -> Sha256Digest {
        self.platform()
            .expect(CUSTODY_PLATFORM_UNAVAILABLE_DETAIL)
            .recovery_identity_sha256()
    }

    pub(crate) async fn send_descriptors_unfenced(
        &self,
        stream: &CustodyUnixStream,
        holder: CustodyHolderIdentity,
    ) -> Result<()> {
        self.prepare_descriptor_send(holder)?;
        let platform = self.platform()?;
        stream.send_rights(&platform.as_descriptors()).await
    }

    pub(crate) fn platform(&self) -> Result<Arc<PlatformCustodyExport>> {
        self.platform
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .ok_or_else(|| anyhow::anyhow!(CUSTODY_PLATFORM_UNAVAILABLE_DETAIL))
    }

    pub(crate) fn prepare_descriptor_send(&self, holder: CustodyHolderIdentity) -> Result<()> {
        if let Some(transfer) = self.startup_transfer() {
            transfer.mark_transfer_possible(holder)?;
        }
        self.inject_initial_fault(InitialCustodyHandshakeBoundary::DescriptorSend)
    }

    pub(crate) fn close_platform_after_successor_exports(&self) {
        drop(
            self.platform
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take(),
        );
    }

    pub(crate) fn inject_initial_fault(
        &self,
        boundary: InitialCustodyHandshakeBoundary,
    ) -> Result<()> {
        #[cfg(test)]
        anyhow::ensure!(
            self.initial_fault != Some(boundary),
            INJECTED_INITIAL_HANDSHAKE_FAILURE_DETAIL
        );
        #[cfg(not(test))]
        let _ = boundary;
        Ok(())
    }

    pub(crate) fn transfer_possible(&self) -> bool {
        self.startup_transfer()
            .is_some_and(|transfer| transfer.transfer_possible())
    }

    pub(crate) fn complete_startup_transfer(&self) {
        *self
            .startup_transfer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    }

    fn startup_transfer(&self) -> Option<Arc<StartupCustodyTransferFence>> {
        self.startup_transfer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}
