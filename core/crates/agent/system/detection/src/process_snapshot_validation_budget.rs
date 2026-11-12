use super::super::constants::{MAX_PROCESS_ARGV_BYTES, MAX_PROCESS_PATH_BYTES, process_error};
use super::limits::{
    MAX_PROCESS_SNAPSHOT_TOTAL_ARGV_ITEMS, MAX_PROCESS_SNAPSHOT_TOTAL_ENVIRONMENT_ITEMS,
    MAX_PROCESS_SNAPSHOT_TOTAL_EVIDENCE_ITEMS,
};
use super::validation::invalid_snapshot;
use anyhow::Result;

#[derive(Default)]
pub(super) struct SnapshotBudget {
    argument_bytes: usize,
    argv_items: usize,
    environment_items: usize,
    evidence_items: usize,
    path_bytes: usize,
}

impl SnapshotBudget {
    pub(super) fn record_arguments(&mut self, items: usize, bytes: usize) -> Result<()> {
        self.argv_items = checked_total(self.argv_items, items)?;
        self.argument_bytes = checked_total(self.argument_bytes, bytes)?;
        anyhow::ensure!(
            self.argv_items <= MAX_PROCESS_SNAPSHOT_TOTAL_ARGV_ITEMS
                && self.argument_bytes <= MAX_PROCESS_ARGV_BYTES,
            process_error::SNAPSHOT_INVALID
        );
        Ok(())
    }

    pub(super) fn record_environment(&mut self, items: usize) -> Result<()> {
        self.environment_items = checked_total(self.environment_items, items)?;
        anyhow::ensure!(
            self.environment_items <= MAX_PROCESS_SNAPSHOT_TOTAL_ENVIRONMENT_ITEMS,
            process_error::SNAPSHOT_INVALID
        );
        Ok(())
    }

    pub(super) fn record_evidence(&mut self, items: usize) -> Result<()> {
        self.evidence_items = checked_total(self.evidence_items, items)?;
        anyhow::ensure!(
            self.evidence_items <= MAX_PROCESS_SNAPSHOT_TOTAL_EVIDENCE_ITEMS,
            process_error::SNAPSHOT_INVALID
        );
        Ok(())
    }

    pub(super) fn record_path(&mut self, bytes: usize) -> Result<()> {
        self.path_bytes = checked_total(self.path_bytes, bytes)?;
        anyhow::ensure!(
            self.path_bytes <= MAX_PROCESS_PATH_BYTES,
            process_error::SNAPSHOT_INVALID
        );
        Ok(())
    }
}

fn checked_total(current: usize, additional: usize) -> Result<usize> {
    current.checked_add(additional).ok_or_else(invalid_snapshot)
}
