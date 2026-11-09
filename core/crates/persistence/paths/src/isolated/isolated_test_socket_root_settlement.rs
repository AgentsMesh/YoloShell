use super::super::{IsolatedTestSocketRoot, create};
use crate::paths::constants::{
    MAX_PRIVATE_DIRECTORY_ENTRIES, SOCKET_LIFECYCLE_LOCK_RETRY_MILLIS,
    SOCKET_LIFECYCLE_LOCK_TIMEOUT, SOCKET_STAGE_SWEEP_ENTRY_BUDGET,
};
use crate::paths::directory_stream::DirectoryStream;
use crate::paths::socket_namespace::SocketNamespaceScanner;
use anyhow::Result;
use std::time::{Duration, Instant};
use yolo_maintenance_contract::{MaintenanceScanner, ScanChunk};

const SETTLEMENT_DISCOVERY_CHUNKS_PER_NAMESPACE: usize = 1;
const SETTLEMENT_STAGE_PROGRESS_CHUNKS_PER_NAMESPACE: usize =
    MAX_PRIVATE_DIRECTORY_ENTRIES.div_ceil(SOCKET_STAGE_SWEEP_ENTRY_BUDGET);
const SETTLEMENT_COMPLETION_CHUNKS_PER_NAMESPACE: usize = 1;
const SETTLEMENT_FINAL_PASS_CHUNKS: usize = 1;
const SETTLEMENT_CHUNKS_PER_NAMESPACE: usize = SETTLEMENT_DISCOVERY_CHUNKS_PER_NAMESPACE
    + SETTLEMENT_STAGE_PROGRESS_CHUNKS_PER_NAMESPACE
    + SETTLEMENT_COMPLETION_CHUNKS_PER_NAMESPACE;
const SETTLEMENT_CHUNK_LIMIT: usize =
    MAX_PRIVATE_DIRECTORY_ENTRIES * SETTLEMENT_CHUNKS_PER_NAMESPACE + SETTLEMENT_FINAL_PASS_CHUNKS;
const SETTLEMENT_DEADLINE_EXHAUSTED: &str =
    "isolated test socket namespace settlement deadline elapsed";
const SETTLEMENT_WORK_EXHAUSTED: &str =
    "isolated test socket namespace settlement scanner work budget exhausted";

pub(super) fn settle(root: &mut IsolatedTestSocketRoot) -> Result<()> {
    settle_with_pause(root, &mut std::thread::sleep)
}

fn settle_with_pause(
    root: &mut IsolatedTestSocketRoot,
    pause: &mut impl FnMut(Duration),
) -> Result<()> {
    create::verify(root)?;
    if !has_namespace_entries(root)? {
        return Ok(());
    }
    let mut scanner = SocketNamespaceScanner::from_root(&root.root, true)?;
    let deadline = Instant::now() + SOCKET_LIFECYCLE_LOCK_TIMEOUT;
    run_bounded_settlement(&mut scanner, deadline, |scanner| {
        create::verify(root)?;
        finish_chunk(
            scanner.scan_chunk()?,
            || {
                create::verify(root)?;
                Ok(!has_namespace_entries(root)?)
            },
            pause,
        )
    })
}

fn finish_chunk(
    chunk: ScanChunk,
    is_empty: impl FnOnce() -> Result<bool>,
    pause: &mut impl FnMut(Duration),
) -> Result<bool> {
    match chunk {
        ScanChunk::PassComplete => {
            if is_empty()? {
                return Ok(true);
            }
            pause(Duration::from_millis(SOCKET_LIFECYCLE_LOCK_RETRY_MILLIS));
        }
        ScanChunk::Progress => std::thread::yield_now(),
        ScanChunk::Busy => pause(Duration::from_millis(SOCKET_LIFECYCLE_LOCK_RETRY_MILLIS)),
    }
    Ok(false)
}

fn run_bounded_settlement<S, F>(state: &mut S, deadline: Instant, mut step: F) -> Result<()>
where
    F: FnMut(&mut S) -> Result<bool>,
{
    for _ in 0..SETTLEMENT_CHUNK_LIMIT {
        anyhow::ensure!(Instant::now() < deadline, SETTLEMENT_DEADLINE_EXHAUSTED);
        if step(state)? {
            return Ok(());
        }
    }
    anyhow::bail!(SETTLEMENT_WORK_EXHAUSTED)
}

fn has_namespace_entries(root: &IsolatedTestSocketRoot) -> Result<bool> {
    let mut stream = DirectoryStream::open(root.root.descriptor())?;
    Ok(stream.next_name()?.is_some())
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../unit_tests/isolated/isolated_test_socket_root_settlement_tests.rs"]
mod unit_tests;

#[cfg(test)]
#[cfg(feature = "yolo-integration-tests")]
#[path = "../../integration_tests/isolated/isolated_test_socket_root_settlement_tests.rs"]
mod integration_tests;
