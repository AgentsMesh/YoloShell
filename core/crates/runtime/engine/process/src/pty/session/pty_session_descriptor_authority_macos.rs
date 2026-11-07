use std::sync::Arc;

use super::super::MacosProcessCapabilityManagerReader;

#[derive(Clone)]
pub(super) struct MacosDescriptorAuthority(Arc<MacosProcessCapabilityManagerReader>);

impl MacosDescriptorAuthority {
    pub(super) fn new(authority: Arc<MacosProcessCapabilityManagerReader>) -> Self {
        Self(authority)
    }

    pub(super) fn writer_handle(&self) -> u64 {
        self.0.writer_handle()
    }

    pub(super) fn process_reader(&self) -> &MacosProcessCapabilityManagerReader {
        &self.0
    }
}
