#[cfg(all(test, feature = "yolo-integration-tests"))]
use super::super::super::registered_cleanup::test_hooks;
use super::super::super::{socket_stage, stage_scan};
use crate::paths::{
    constants::{MAX_PRIVATE_DIRECTORY_ENTRIES, SOCKET_STAGE_RETIRED_CHUNK_LIMIT},
    directory::{PrivateDirectory, entry_c_string},
    directory_stream::DirectoryStream,
};
use anyhow::Result;
use yolo_maintenance_contract::ScanChunk;

/// The caller retains directory EX and root lifecycle. Endpoint and unknown entries
/// reject the entire pass before any publication bookkeeping is inspected for cleanup.
pub(super) fn settle(directory: &PrivateDirectory, fence: impl Fn() -> Result<()>) -> Result<bool> {
    fence()?;
    if !only_bookkeeping(directory)? {
        return Ok(false);
    }
    let mut scanner = stage_scan::StageScanner::new(directory.try_clone()?);
    for _ in 0..SOCKET_STAGE_RETIRED_CHUNK_LIMIT {
        fence()?;
        let result = scanner.scan_chunk()?;
        #[cfg(all(test, feature = "yolo-integration-tests"))]
        test_hooks::run(test_hooks::Point::AfterOrphanStageChunk);
        fence()?;
        match result {
            ScanChunk::Progress => {}
            ScanChunk::Busy => return Ok(false),
            ScanChunk::PassComplete => return Ok(true),
        }
    }
    Ok(false)
}

fn only_bookkeeping(directory: &PrivateDirectory) -> Result<bool> {
    let mut entries = DirectoryStream::open(directory.descriptor())?;
    for _ in 0..MAX_PRIVATE_DIRECTORY_ENTRIES {
        let Some(name) = entries.next_name()? else {
            return Ok(true);
        };
        let name = entry_c_string(&name)?;
        if !socket_stage::is_lifecycle(&name)
            && socket_stage::lease_key(&name).is_none()
            && !socket_stage::is_managed_stage(&name)
            && !socket_stage::cleanup::is_unlink_isolation(&name)
        {
            return Ok(false);
        }
    }
    Ok(false)
}
