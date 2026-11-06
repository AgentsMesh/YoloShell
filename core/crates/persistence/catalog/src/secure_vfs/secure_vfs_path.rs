use rusqlite::ffi;
use std::ffi::{CStr, OsString};
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

pub(super) unsafe extern "C" fn delete(
    wrapper: *mut ffi::sqlite3_vfs,
    name: *const libc::c_char,
    sync_directory: libc::c_int,
) -> libc::c_int {
    let Some(path) = c_path(name) else {
        return ffi::SQLITE_IOERR_DELETE;
    };
    match super::retained_namespace::resolve(&path) {
        Some(entry) => match super::retained_delete::remove_artifact(&entry) {
            Ok(()) => ffi::SQLITE_OK,
            Err(error) => retained_error("delete", &path, error, ffi::SQLITE_IOERR_DELETE),
        },
        None => unsafe {
            delegate(wrapper, |base| {
                (*base).xDelete.map_or(ffi::SQLITE_IOERR_DELETE, |delete| {
                    delete(base, name, sync_directory)
                })
            })
        },
    }
}

pub(super) unsafe extern "C" fn access(
    wrapper: *mut ffi::sqlite3_vfs,
    name: *const libc::c_char,
    flags: libc::c_int,
    output: *mut libc::c_int,
) -> libc::c_int {
    let Some(path) = c_path(name) else {
        return ffi::SQLITE_IOERR_ACCESS;
    };
    match super::retained_namespace::resolve(&path) {
        Some(entry) => match super::retained_access::access(&entry, flags) {
            Ok(accessible) => {
                if !output.is_null() {
                    unsafe { *output = libc::c_int::from(accessible) };
                }
                ffi::SQLITE_OK
            }
            Err(error) => retained_error("access", &path, error, ffi::SQLITE_IOERR_ACCESS),
        },
        None => unsafe {
            delegate(wrapper, |base| {
                (*base).xAccess.map_or(ffi::SQLITE_IOERR_ACCESS, |access| {
                    access(base, name, flags, output)
                })
            })
        },
    }
}

pub(super) unsafe extern "C" fn full_pathname(
    wrapper: *mut ffi::sqlite3_vfs,
    name: *const libc::c_char,
    output_capacity: libc::c_int,
    output: *mut libc::c_char,
) -> libc::c_int {
    let Some(path) = c_path(name) else {
        return ffi::SQLITE_CANTOPEN;
    };
    if super::retained_namespace::is_registered(&path) {
        let bytes = unsafe { CStr::from_ptr(name) }.to_bytes();
        let required = match libc::c_int::try_from(bytes.len() + 1) {
            Ok(required) => required,
            Err(_) => return ffi::SQLITE_CANTOPEN,
        };
        if output.is_null() || output_capacity < required {
            return ffi::SQLITE_CANTOPEN;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr().cast(), output, bytes.len());
            *output.add(bytes.len()) = libc::c_char::default();
        }
        return ffi::SQLITE_OK;
    }
    unsafe {
        delegate(wrapper, |base| {
            (*base)
                .xFullPathname
                .map_or(ffi::SQLITE_CANTOPEN, |full_pathname| {
                    full_pathname(base, name, output_capacity, output)
                })
        })
    }
}

fn c_path(name: *const libc::c_char) -> Option<PathBuf> {
    if name.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(name) }.to_bytes().to_vec();
    Some(PathBuf::from(OsString::from_vec(bytes)))
}

unsafe fn delegate<F>(wrapper: *mut ffi::sqlite3_vfs, action: F) -> libc::c_int
where
    F: FnOnce(*mut ffi::sqlite3_vfs) -> libc::c_int,
{
    let base = unsafe { (*wrapper).pAppData.cast::<ffi::sqlite3_vfs>() };
    action(base)
}

fn retained_error(
    operation: &str,
    path: &Path,
    error: anyhow::Error,
    code: libc::c_int,
) -> libc::c_int {
    super::secure_vfs::record_violation(format!(
        "retained SQLite {operation} failed for {}: {error:#}",
        path.display()
    ));
    code
}
