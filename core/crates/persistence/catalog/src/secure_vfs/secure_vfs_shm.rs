use super::secure_vfs_file::{ArtifactKind, SecureDescriptor};
use anyhow::Result;
use rusqlite::ffi;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

const SQLITE_SHM_SUFFIX: &str = "-shm";
const SHM_OPEN_FLAGS: libc::c_int =
    ffi::SQLITE_OPEN_READWRITE | ffi::SQLITE_OPEN_CREATE | ffi::SQLITE_OPEN_WAL;
const PRESERVE_NATIVE_SHM: libc::c_int = 0;

pub(super) unsafe extern "C" fn map(
    file: *mut ffi::sqlite3_file,
    page: libc::c_int,
    page_size: libc::c_int,
    extend: libc::c_int,
    output: *mut *mut libc::c_void,
) -> libc::c_int {
    match unsafe { map_inner(file, page, page_size, extend, output) } {
        Ok(result) => result,
        Err(error) => super::secure_vfs::callback_error(error),
    }
}

unsafe fn map_inner(
    file: *mut ffi::sqlite3_file,
    page: libc::c_int,
    page_size: libc::c_int,
    extend: libc::c_int,
    output: *mut *mut libc::c_void,
) -> Result<libc::c_int> {
    let (alias_path, artifact_path, methods) = super::secure_vfs_state::state_snapshot(file)
        .ok_or_else(|| anyhow::anyhow!("SQLite SHM file state disappeared"))?;
    let shm_path = append_suffix(&artifact_path, SQLITE_SHM_SUFFIX);
    let name = CString::new(shm_path.as_os_str().as_bytes())?;
    let descriptor = SecureDescriptor::open(&name, SHM_OPEN_FLAGS)?;
    super::secure_vfs_state::verify_shm_descriptor(file, &descriptor)?;
    super::secure_vfs::run_test_hook(ArtifactKind::Shm, descriptor.path());
    super::secure_vfs_state::create_shm_link(file, &descriptor)?;
    if let Err(error) = super::secure_vfs_state::set_alias_sealed(file, true) {
        let _ = cleanup_alias(file, &descriptor);
        return Err(error);
    }
    super::secure_vfs::run_alias_test_hook(ArtifactKind::Shm, &alias_path);
    if let Err(error) = descriptor.verify_alias(&alias_path) {
        cleanup_alias(file, &descriptor)?;
        return Err(error);
    }
    let methods = methods as *const ffi::sqlite3_io_methods;
    let result = unsafe {
        (*methods).xShmMap.map_or(ffi::SQLITE_IOERR_SHMOPEN, |map| {
            map(file, page, page_size, extend, output)
        })
    };
    let alias_result = descriptor.verify_alias(&alias_path);
    let cleanup_result = cleanup_alias(file, &descriptor);
    alias_result.and(cleanup_result)?;
    descriptor.verify_current()?;
    let identity = descriptor.identity();
    if super::secure_vfs_state::retain_shm_descriptor(file, descriptor)? {
        if let Err(error) = super::secure_vfs_shm_registry::retain(identity) {
            let _ = super::secure_vfs_state::take_shm_descriptor(file);
            return Err(error);
        }
    }
    Ok(result)
}

pub(super) unsafe extern "C" fn unmap(
    file: *mut ffi::sqlite3_file,
    delete_flag: libc::c_int,
) -> libc::c_int {
    match unsafe { unmap_inner(file, delete_flag) } {
        Ok(result) => result,
        Err(error) => super::secure_vfs::callback_error(error),
    }
}

unsafe fn unmap_inner(
    file: *mut ffi::sqlite3_file,
    delete_flag: libc::c_int,
) -> Result<libc::c_int> {
    let (_, _, methods) = super::secure_vfs_state::state_snapshot(file)
        .ok_or_else(|| anyhow::anyhow!("SQLite SHM file state disappeared"))?;
    let methods = methods as *const ffi::sqlite3_io_methods;
    let result = unsafe {
        (*methods)
            .xShmUnmap
            .map_or(ffi::SQLITE_IOERR_SHMOPEN, |unmap| {
                unmap(file, PRESERVE_NATIVE_SHM)
            })
    };
    if result != ffi::SQLITE_OK {
        return Ok(result);
    }
    let descriptor = super::secure_vfs_state::take_shm_descriptor(file)?;
    if let Some(descriptor) = descriptor {
        let delete = super::secure_vfs_shm_registry::release(
            descriptor.identity(),
            delete_flag != PRESERVE_NATIVE_SHM,
        )?;
        if delete {
            descriptor.delete_current()?;
        }
    }
    Ok(result)
}

fn cleanup_alias(file: *mut ffi::sqlite3_file, descriptor: &SecureDescriptor) -> Result<()> {
    let mode_result = super::secure_vfs_state::set_alias_sealed(file, false);
    let remove_result = super::secure_vfs_state::remove_shm_link(file, descriptor);
    mode_result.and(remove_result)
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
