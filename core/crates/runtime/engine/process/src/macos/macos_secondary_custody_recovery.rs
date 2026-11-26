use super::{MacosProcessCapabilityIdentity, MacosSecondaryCustody};
use crate::pty_custody::CustodyDescriptorSet;
use anyhow::Result;
use std::os::fd::OwnedFd;

impl MacosSecondaryCustody {
    #[doc(hidden)]
    pub fn duplicate_recovery_reader(&self) -> Result<OwnedFd> {
        self.process_reader.duplicate_for_secondary()
    }

    pub(crate) fn process_capability_identity(&self) -> MacosProcessCapabilityIdentity {
        self.process_reader.identity()
    }

    #[doc(hidden)]
    pub fn matches_process_descriptor_set(&self, set: CustodyDescriptorSet) -> bool {
        match set {
            CustodyDescriptorSet::MacosProcessCapability {
                reader_handle,
                writer_handle,
            } => {
                let identity = self.process_capability_identity();
                identity.reader_handle() == reader_handle
                    && identity.writer_handle() == writer_handle
            }
            _ => false,
        }
    }
}
