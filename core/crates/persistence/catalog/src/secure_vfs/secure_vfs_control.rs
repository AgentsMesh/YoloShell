use rusqlite::ffi;

pub(super) const FILE_NOT_MOVED: libc::c_int = 0;

pub(super) unsafe extern "C" fn file_control(
    file: *mut ffi::sqlite3_file,
    operation: libc::c_int,
    argument: *mut libc::c_void,
) -> libc::c_int {
    if operation == ffi::SQLITE_FCNTL_HAS_MOVED {
        return retained_has_moved(file, argument);
    }
    let Some(methods) = super::secure_vfs_state::original_methods(file) else {
        return ffi::SQLITE_IOERR;
    };
    let methods = methods as *const ffi::sqlite3_io_methods;
    unsafe {
        (*methods)
            .xFileControl
            .map_or(ffi::SQLITE_NOTFOUND, |control| {
                control(file, operation, argument)
            })
    }
}

fn retained_has_moved(file: *mut ffi::sqlite3_file, argument: *mut libc::c_void) -> libc::c_int {
    if argument.is_null() {
        super::secure_vfs::record_violation("SQLite HAS_MOVED output is null".to_owned());
        return ffi::SQLITE_IOERR;
    }
    if let Err(error) = super::secure_vfs_state::verify_retained(file) {
        super::secure_vfs::record_violation(format!(
            "retained SQLite HAS_MOVED verification failed: {error:#}"
        ));
        return ffi::SQLITE_IOERR;
    }
    unsafe { *argument.cast::<libc::c_int>() = FILE_NOT_MOVED };
    ffi::SQLITE_OK
}
