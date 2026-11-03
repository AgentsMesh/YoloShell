use crate::agent::constants::{
    JSONL_EXTENSION, MAX_PROCESS_EVIDENCE_PATHS, MIN_PROCESS_PID, PROCESS_FD_SENTINEL_COUNT, macos,
    process_error,
};
use anyhow::{Context, Result, ensure};
use std::ffi::OsString;
use std::mem::{MaybeUninit, size_of, size_of_val};
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

#[repr(C)]
struct ProcessFileInfo {
    open_flags: u32,
    status: u32,
    offset: libc::off_t,
    file_type: i32,
    guard_flags: u32,
}

#[repr(C)]
struct VnodeFdInfoWithPath {
    file: ProcessFileInfo,
    vnode: libc::vnode_info_path,
}

pub(super) fn paths(pid: u32, descriptor_limit: usize) -> Result<Vec<PathBuf>> {
    let pid = checked_pid(pid)?;
    let descriptors = list_descriptors(pid, descriptor_limit)?;
    let vnode_type =
        u32::try_from(libc::PROX_FDTYPE_VNODE).context(process_error::VNODE_TYPE_INVALID)?;
    let mut paths = Vec::new();
    for descriptor in descriptors
        .into_iter()
        .filter(|value| value.proc_fdtype == vnode_type)
    {
        let Some(path) = vnode_path(pid, descriptor.proc_fd) else {
            continue;
        };
        if path.extension().and_then(|value| value.to_str()) != Some(JSONL_EXTENSION) {
            continue;
        }
        if paths.contains(&path) {
            continue;
        }
        ensure!(
            paths.len() < MAX_PROCESS_EVIDENCE_PATHS,
            process_error::EVIDENCE_PATH_LIMIT
        );
        paths.push(path);
    }
    paths.sort();
    Ok(paths)
}

fn list_descriptors(pid: i32, limit: usize) -> Result<Vec<libc::proc_fdinfo>> {
    let capacity = limit
        .checked_add(PROCESS_FD_SENTINEL_COUNT)
        .context(process_error::FD_BUFFER_OVERFLOW)?;
    let bytes = capacity
        .checked_mul(size_of::<libc::proc_fdinfo>())
        .context(process_error::FD_BUFFER_OVERFLOW)?;
    let bytes = i32::try_from(bytes).context(process_error::FD_BUFFER_RANGE)?;
    let mut values = (0..capacity)
        .map(|_| unsafe { std::mem::zeroed() })
        .collect::<Vec<libc::proc_fdinfo>>();
    let read = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDLISTFDS,
            0,
            values.as_mut_ptr().cast(),
            bytes,
        )
    };
    ensure!(read > 0, process_error::FD_INVENTORY_FAILED);
    let read = read as usize;
    ensure!(
        read.is_multiple_of(size_of::<libc::proc_fdinfo>()),
        process_error::FD_INVENTORY_MISALIGNED
    );
    let count = read / size_of::<libc::proc_fdinfo>();
    ensure!(count <= limit, process_error::FD_INVENTORY_LIMIT);
    values.truncate(count);
    Ok(values)
}

fn vnode_path(pid: i32, descriptor: i32) -> Option<PathBuf> {
    let mut value = MaybeUninit::<VnodeFdInfoWithPath>::uninit();
    let expected = i32::try_from(size_of::<VnodeFdInfoWithPath>()).ok()?;
    let read = unsafe {
        libc::proc_pidfdinfo(
            pid,
            descriptor,
            macos::VNODE_PATH_INFO_FLAVOR,
            value.as_mut_ptr().cast(),
            expected,
        )
    };
    if read != expected {
        return None;
    }
    let value = unsafe { value.assume_init() };
    let raw = &value.vnode.vip_path;
    let bytes = unsafe { std::slice::from_raw_parts(raw.as_ptr().cast(), size_of_val(raw)) };
    path_from_c_bytes(bytes)
}

fn checked_pid(pid: u32) -> Result<i32> {
    ensure!(pid >= MIN_PROCESS_PID, process_error::PID_INVALID);
    i32::try_from(pid).context(process_error::PID_INVALID)
}

fn path_from_c_bytes(bytes: &[u8]) -> Option<PathBuf> {
    let end = bytes.iter().position(|byte| *byte == 0)?;
    (end > 0).then(|| PathBuf::from(OsString::from_vec(bytes[..end].to_vec())))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/macos_evidence_tests.rs"]
mod tests;
