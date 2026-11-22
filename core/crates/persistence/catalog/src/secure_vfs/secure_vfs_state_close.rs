unsafe extern "C" fn secure_close(file: *mut ffi::sqlite3_file) -> libc::c_int {
    let Some(mut state) = lock_states().remove(&(file as usize)) else {
        return ffi::SQLITE_IOERR_CLOSE;
    };
    let original = state.original_methods as *const ffi::sqlite3_io_methods;
    unsafe { (*file).pMethods = original };
    let result = unsafe {
        (*original)
            .xClose
            .map_or(ffi::SQLITE_IOERR_CLOSE, |close| close(file))
    };
    let cleanup = state.shm_descriptor.take().map_or(Ok(()), |descriptor| {
        super::secure_vfs_shm_registry::release(descriptor.identity(), false).and_then(|delete| {
            if delete {
                descriptor.delete_current()
            } else {
                Ok(())
            }
        })
    });
    drop(state);
    if result != ffi::SQLITE_OK {
        result
    } else if let Err(error) = cleanup {
        super::secure_vfs::callback_error(error)
    } else {
        result
    }
}
