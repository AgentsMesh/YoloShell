use super::captured_lineage::LineageEdge;
use super::constants::NO_PARENT_LINEAGE_ID;
use super::{LiveProcessIdentity, ProcessIdentity};
use anyhow::Result;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct InventoryEntry {
    pub(super) pid: u32,
    pub(super) parent_pid: u32,
    pub(super) process_group: u32,
    pub(super) session_id: u32,
    pub(super) start_time: u64,
    #[cfg(target_os = "linux")]
    pub(super) effective_user_id: u32,
    #[cfg(target_os = "macos")]
    pub(super) unique_id: u64,
    #[cfg(target_os = "macos")]
    pub(super) parent_unique_id: u64,
}

impl InventoryEntry {
    pub(super) fn validate_identity(self) -> Result<()> {
        #[cfg(target_os = "macos")]
        LiveProcessIdentity::from_kernel(self.pid, self.unique_id).map(|_identity| ())?;
        Ok(())
    }

    pub(super) fn identity(self) -> ProcessIdentity {
        ProcessIdentity {
            pid: self.pid,
            start_time: self.start_time,
        }
    }

    #[cfg(target_os = "linux")]
    pub(super) fn live_identity(self) -> Result<Option<LiveProcessIdentity>> {
        LiveProcessIdentity::capture(self.identity())
    }

    #[cfg(not(target_os = "linux"))]
    pub(super) fn live_identity(self) -> Result<LiveProcessIdentity> {
        #[cfg(target_os = "macos")]
        return LiveProcessIdentity::from_kernel(self.pid, self.unique_id);
        #[cfg(not(target_os = "macos"))]
        return Ok(LiveProcessIdentity::from_portable(self.identity()));
    }

    pub(super) fn lineage_id(self) -> u64 {
        #[cfg(target_os = "macos")]
        {
            self.unique_id
        }
        #[cfg(not(target_os = "macos"))]
        {
            u64::from(self.pid)
        }
    }

    pub(super) fn parent_lineage_id(self) -> u64 {
        #[cfg(target_os = "macos")]
        {
            self.parent_unique_id
        }
        #[cfg(not(target_os = "macos"))]
        {
            u64::from(self.parent_pid)
        }
    }

    pub(super) fn lineage_edge(self) -> Option<LineageEdge> {
        let parent = self.parent_lineage_id();
        (parent != NO_PARENT_LINEAGE_ID).then_some(LineageEdge {
            child: self.lineage_id(),
            parent,
        })
    }

    pub(super) fn stable_with(self, other: Self) -> bool {
        #[cfg(target_os = "macos")]
        {
            self.pid == other.pid
                && self.process_group == other.process_group
                && self.session_id == other.session_id
                && self.start_time == other.start_time
                && self.unique_id == other.unique_id
                && self.parent_unique_id == other.parent_unique_id
        }
        #[cfg(not(target_os = "macos"))]
        {
            self == other
        }
    }
}
