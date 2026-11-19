use super::PtySessionProof;
use crate::process::{ProcessIdentity, inventory_entry::InventoryEntry};
use anyhow::{Result, ensure};

const INVALID_ORIGIN: &str = "Linux capability origin is newer than the execution root";

impl PtySessionProof {
    pub(in crate::process) fn bind_linux_capability_origin(
        &mut self,
        origin: Option<ProcessIdentity>,
        root: ProcessIdentity,
    ) -> Result<()> {
        ensure!(
            origin.is_none_or(|origin| origin.start_time != u64::default()
                && origin.start_time <= root.start_time),
            INVALID_ORIGIN
        );
        self.bootstrap.linux_origin = origin;
        Ok(())
    }

    /// Fresh capabilities are inherited by processes created by their issuer.
    /// The issuer's birth is deliberately earlier than issuance: siblings born
    /// before the root, coarse same-tick identities and escaped descendants stay eligible.
    pub(super) fn linux_candidate_relevant(
        &self,
        entry: InventoryEntry,
        root: ProcessIdentity,
        retained: bool,
    ) -> bool {
        self.bootstrap.linux_origin.is_none_or(|origin| {
            origin.start_time == u64::default()
                || origin.start_time > root.start_time
                || entry.start_time == u64::default()
                || entry.start_time >= origin.start_time
                || retained
                || entry.pid == root.pid
                || entry.parent_pid == root.pid
                || entry.session_id == root.pid
        })
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../../unit_tests/pty_session_linux_origin_tests.rs"]
mod tests;
