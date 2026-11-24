use super::LiveProcessIdentity;
use anyhow::Result;

impl LiveProcessIdentity {
    pub fn macos_unique_id_for_test(&self) -> u64 {
        self.unique_id()
    }

    pub fn from_macos_unique_id_for_test(pid: u32, unique_id: u64) -> Result<Self> {
        Self::from_kernel(pid, unique_id)
    }
}
