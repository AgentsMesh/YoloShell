use super::*;
use crate::paths::directory::entry_c_string;
use anyhow::Result;
use yolo_maintenance_contract::ScanChunk;
#[path = "socket_namespace_certified_endpoints.rs"]
mod certified_endpoints;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../test_support/integration/socket_namespace_cleanup_hooks.rs"]
pub(super) mod test_hooks;

pub(super) fn cleanup(namespace: &mut SocketNamespace) -> Result<NamespaceRelease> {
    namespace.verify()?;
    let mut lifecycle = lifecycle_lock::SocketNamespaceLifecycleLock::acquire(&namespace.root)?;
    namespace.verify()?;
    if !namespace.authority.registration.try_upgrade()? {
        return Ok(NamespaceRelease::Busy);
    }
    if !namespace
        .authority
        .try_upgrade_directory(&namespace.directory)?
    {
        return Ok(NamespaceRelease::Busy);
    }
    if !only_known_entries(&namespace.directory)? {
        return Ok(NamespaceRelease::Preserved);
    }
    if !clean_bookkeeping(&namespace.directory)? {
        return Ok(NamespaceRelease::Preserved);
    }
    if !certified_endpoints::cleanup(&namespace.directory, || namespace.verify())? {
        return Ok(NamespaceRelease::Preserved);
    }
    namespace.verify()?;
    let Some(name) = namespace.location.directory.file_name() else {
        return Ok(NamespaceRelease::Preserved);
    };
    let name = entry_c_string(name)?;
    // Recheck the retained directory identity and actual emptiness after proven endpoint cleanup.
    Ok(
        if empty_remove::owned_locked(&namespace.root, &namespace.directory, &name, &mut lifecycle)?
        {
            NamespaceRelease::Removed
        } else {
            NamespaceRelease::Preserved
        },
    )
}

fn only_known_entries(directory: &PrivateDirectory) -> Result<bool> {
    let mut entries = stream::DirectoryStream::open(directory.descriptor())?;
    for _ in 0..MAX_PRIVATE_DIRECTORY_ENTRIES {
        let Some(name) = entries.next_name()? else {
            return Ok(true);
        };
        let name = entry_c_string(&name)?;
        if socket_stage::is_lifecycle(&name)
            || socket_stage::lease_key(&name).is_some()
            || socket_stage::is_managed_stage(&name)
            || socket_stage::cleanup::is_unlink_isolation(&name)
            || socket_path::is_endpoint_name(&name)
        {
            continue;
        }
        return Ok(false);
    }
    Ok(false)
}

fn clean_bookkeeping(directory: &PrivateDirectory) -> Result<bool> {
    let mut scanner = stage_scan::StageScanner::new(directory.try_clone()?);
    let mut complete = false;
    for _ in 0..SOCKET_STAGE_RETIRED_CHUNK_LIMIT {
        let chunk = scanner.scan_chunk()?;
        #[cfg(all(test, feature = "yolo-integration-tests"))]
        test_hooks::run(test_hooks::Point::AfterRegisteredStageChunk);
        match chunk {
            ScanChunk::Progress => {}
            ScanChunk::Busy => return Ok(false),
            ScanChunk::PassComplete => {
                complete = true;
                break;
            }
        }
    }
    if !complete {
        return Ok(false);
    }
    Ok(matches!(
        socket_stage::cleanup_lifecycle(directory)?,
        socket_stage::Cleanup::Mutated | socket_stage::Cleanup::Unchanged
    ))
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/socket_namespace/socket_namespace_registered_cleanup_tests.rs"]
mod tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/socket_namespace/socket_namespace_cleanup_work_bounds_tests.rs"]
mod work_bounds_tests;
