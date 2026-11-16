use crate::paths::identity::DurableFileVersion;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::Result;

pub(super) trait DurableTestRemoval {
    fn remove_if_durable_exact(&self, expected: DurableFileVersion, max_bytes: u64)
    -> Result<bool>;
}

impl DurableTestRemoval for PrivateFileEntry {
    fn remove_if_durable_exact(
        &self,
        expected: DurableFileVersion,
        max_bytes: u64,
    ) -> Result<bool> {
        let live = super::durable_io::strict(self, expected)?;
        self.remove_if_exact(live, max_bytes)
    }
}
