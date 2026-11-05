#[cfg(target_os = "macos")]
use super::constants::MAX_PROCESS_INVENTORY_IDS;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
use super::constants::{PROCESS_INVENTORY_ARGS, PROCESS_INVENTORY_PROGRAM};
pub(super) use super::inventory_entry::InventoryEntry;
#[cfg(target_os = "linux")]
#[path = "inventory_snapshot_linux.rs"]
mod linux;
#[cfg(target_os = "macos")]
use super::macos_native;
#[cfg(all(not(target_os = "macos"), any(test, not(target_os = "linux"))))]
use super::process_start_time;
#[cfg(not(target_os = "linux"))]
use anyhow::Context;
use anyhow::Result;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
use anyhow::ensure;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
use std::process::{Command, Stdio};
#[cfg(any(test, feature = "runtime-component-tests"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../test_support/inventory/inventory_snapshot_fixture.rs"]
mod fixture;
#[cfg(any(test, feature = "runtime-component-tests"))]
use fixture::take_test_inventory;
#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) use fixture::{
    TestInventoryGuard, install_repeating_test_inventory, install_test_inventory,
    install_test_inventory_results, is_descendant, parent_map,
};
#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/inventory/inventory_snapshot_branch_matrix_tests.rs"]
mod branch_matrix;
#[cfg(all(test, target_os = "macos"))]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../unit_tests/inventory_snapshot_instance_matrix.rs"]
mod instance_matrix;
#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/inventory/inventory_snapshot_tests.rs"]
mod tests;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
const INVENTORY_RUN_CONTEXT: &str = "run PTY process inventory";
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
const INVENTORY_EXIT_DETAIL: &str = "PTY process inventory exited";
pub(super) fn inventory() -> Result<Vec<InventoryEntry>> {
    #[cfg(any(test, feature = "runtime-component-tests"))]
    if let Some(snapshot) = take_test_inventory() {
        return snapshot.map_err(Into::into);
    }
    #[cfg(target_os = "macos")]
    {
        // SAFETY: geteuid has no pointer arguments or preconditions.
        let owner = unsafe { libc::geteuid() };
        let pids = macos_native::user_pids(owner, MAX_PROCESS_INVENTORY_IDS)?;
        pids.into_iter().filter_map(native_entry).collect()
    }
    #[cfg(target_os = "linux")]
    {
        linux::inventory()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        #[cfg(unix)]
        let _spawn_guard = super::waitable_child_spawn_guard()?;
        let child = Command::new(PROCESS_INVENTORY_PROGRAM)
            .args(PROCESS_INVENTORY_ARGS)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context(INVENTORY_RUN_CONTEXT)?;
        let scanner_pid = child.id();
        let output = child.wait_with_output().context(INVENTORY_RUN_CONTEXT)?;
        ensure!(
            output.status.success(),
            "{} with {}",
            INVENTORY_EXIT_DETAIL,
            output.status
        );
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(parse_inventory_line)
            .filter(|entry| entry.pid != scanner_pid)
            .collect())
    }
}
#[cfg(any(test, not(any(target_os = "linux", target_os = "macos"))))]
pub(super) fn parse_inventory_line(line: &str) -> Option<InventoryEntry> {
    let mut fields = line.split_whitespace();
    let pid = fields.next()?.parse().ok()?;
    #[cfg(target_os = "macos")]
    let (metadata, session_id) = fenced_session(pid).ok()??;
    #[cfg(not(target_os = "macos"))]
    let session_id = process_session_id(pid)?;
    #[cfg(target_os = "macos")]
    let start_time = metadata.start_time;
    #[cfg(not(target_os = "macos"))]
    let start_time = process_start_time(pid).ok()?;
    Some(InventoryEntry {
        pid,
        parent_pid: fields.next()?.parse().ok()?,
        process_group: fields.next()?.parse().ok()?,
        session_id,
        start_time,
        #[cfg(target_os = "linux")]
        effective_user_id: fields.next()?.parse().ok()?,
        #[cfg(target_os = "macos")]
        unique_id: metadata.unique_id,
        #[cfg(target_os = "macos")]
        parent_unique_id: metadata.parent_unique_id,
    })
}
#[cfg(target_os = "macos")]
fn native_entry(pid: u32) -> Option<Result<InventoryEntry>> {
    native_entry_with(
        pid,
        macos_native::try_kernel_process_metadata,
        process_session_id,
    )
}
#[cfg(target_os = "macos")]
fn native_entry_with(
    pid: u32,
    mut metadata: impl FnMut(u32) -> Result<Option<macos_native::KernelProcessMetadata>>,
    mut session: impl FnMut(u32) -> Result<Option<u32>>,
) -> Option<Result<InventoryEntry>> {
    let (info, session_id) = match fenced_session_with(pid, &mut metadata, &mut session) {
        Ok(Some(entry)) => entry,
        Ok(None) => return None,
        Err(error) => return Some(Err(error)),
    };
    let process_group = match u32::try_from(info.process_group) {
        Ok(process_group) => process_group,
        Err(error) => return Some(Err(error).context(PROCESS_GROUP_CONTEXT)),
    };
    Some(Ok(InventoryEntry {
        pid: info.pid,
        parent_pid: info.parent_pid,
        process_group,
        session_id,
        start_time: info.start_time,
        unique_id: info.unique_id,
        parent_unique_id: info.parent_unique_id,
    }))
}
#[cfg(all(test, target_os = "macos"))]
fn fenced_session(pid: u32) -> Result<Option<(macos_native::KernelProcessMetadata, u32)>> {
    fenced_session_with(
        pid,
        macos_native::try_kernel_process_metadata,
        process_session_id,
    )
}
#[cfg(target_os = "macos")]
fn fenced_session_with(
    pid: u32,
    mut metadata: impl FnMut(u32) -> Result<Option<macos_native::KernelProcessMetadata>>,
    mut session: impl FnMut(u32) -> Result<Option<u32>>,
) -> Result<Option<(macos_native::KernelProcessMetadata, u32)>> {
    let Some(before) = metadata(pid)? else {
        return Ok(None);
    };
    let Some(session_id) = session(before.pid)? else {
        return Ok(None);
    };
    let Some(after) = metadata(pid)? else {
        return Ok(None);
    };
    Ok(
        ((before.pid, before.unique_id) == (after.pid, after.unique_id))
            .then_some((after, session_id)),
    )
}
#[cfg(target_os = "macos")]
fn process_session_id(pid: u32) -> Result<Option<u32>> {
    let pid = i32::try_from(pid).context(SESSION_ID_CONTEXT)?;
    let session = unsafe { libc::getsid(pid) };
    if session >= MIN_VALID_SESSION_ID {
        return Ok(Some(session as u32));
    }
    let error = std::io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        return Ok(None);
    }
    Err(error).context(SESSION_ID_CONTEXT)
}
#[cfg(all(not(target_os = "macos"), any(test, not(target_os = "linux"))))]
fn process_session_id(pid: u32) -> Option<u32> {
    let pid = i32::try_from(pid).ok()?;
    let session = unsafe { libc::getsid(pid) };
    (session > MIN_VALID_SESSION_ID).then_some(session as u32)
}
#[cfg(target_os = "macos")]
const PROCESS_GROUP_CONTEXT: &str = "convert macOS process group identity";
#[cfg(target_os = "macos")]
const SESSION_ID_CONTEXT: &str = "inspect macOS process session identity";
const MIN_VALID_SESSION_ID: libc::pid_t = 0;
