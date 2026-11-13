use super::*;
use std::path::Path;

const MAX_TEST_BUSY_RETRIES: usize = 1_024;

pub(in crate::coordination) fn sweep(parent: &Path) -> Result<SweepWork> {
    sweep_pass_for_test(parent, SWEEP_ENTRY_BUDGET, SWEEP_DELETE_BUDGET)
}

pub(in crate::coordination) fn sweep_for_test(
    parent: &Path,
    entries: usize,
    deletions: usize,
) -> Result<SweepWork> {
    let root = PrivateDirectory::open(parent)?;
    CoordinationScanner::from_root(root)
        .scan_with_limits(entries, deletions)
        .map(|value| value.1)
}

pub(in crate::coordination) fn sweep_pass_for_test(
    parent: &Path,
    entries: usize,
    deletions: usize,
) -> Result<SweepWork> {
    let root = PrivateDirectory::open(parent)?;
    let mut scanner = CoordinationScanner::from_root(root);
    let mut total = SweepWork::default();
    let mut busy_retries = 0;
    loop {
        let (progress, work) = scanner.scan_with_limits(entries, deletions)?;
        total.inspected += work.inspected;
        total.probed += work.probed;
        total.deleted += work.deleted;
        match progress {
            ScanChunk::PassComplete => return Ok(total),
            ScanChunk::Busy => {
                busy_retries += 1;
                anyhow::ensure!(
                    busy_retries <= MAX_TEST_BUSY_RETRIES,
                    "test scanner remained busy"
                );
                std::thread::yield_now();
            }
            ScanChunk::Progress => {}
        }
    }
}
