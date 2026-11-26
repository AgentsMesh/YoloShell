const MISSING_METHODS_DETAIL: &str = "SQLite VFS installed no I/O methods";
pub(super) const DUPLICATE_STATE_DETAIL: &str = "SQLite file state pointer is already active";
const REJECTION_ERROR_MISSING_DETAIL: &str = "SQLite install rejection error is present";

pub(super) struct RejectedInstall {
    file: *mut ffi::sqlite3_file,
    error: Option<anyhow::Error>,
    alias: Option<SecureAlias>,
    descriptor: Option<SecureDescriptor>,
}

impl RejectedInstall {
    fn new(
        file: *mut ffi::sqlite3_file,
        error: anyhow::Error,
        alias: SecureAlias,
        descriptor: SecureDescriptor,
    ) -> Self {
        Self {
            file,
            error: Some(error),
            alias: Some(alias),
            descriptor: Some(descriptor),
        }
    }

    pub(super) fn close(mut self) -> anyhow::Error {
        self.close_file();
        self.error.take().expect(REJECTION_ERROR_MISSING_DETAIL)
    }

    fn close_file(&mut self) {
        if self.alias.is_none() {
            return;
        }
        unsafe { super::secure_vfs::close_after_rejection(self.file) };
        self.alias = None;
        self.descriptor = None;
    }
}

impl Drop for RejectedInstall {
    fn drop(&mut self) {
        self.close_file();
    }
}

pub(super) unsafe fn install(
    file: *mut ffi::sqlite3_file,
    alias: SecureAlias,
    descriptor: SecureDescriptor,
    kind: ArtifactKind,
) -> Result<(), RejectedInstall> {
    let original = unsafe { (*file).pMethods };
    if original.is_null() {
        return Err(RejectedInstall::new(
            file,
            anyhow::anyhow!(MISSING_METHODS_DETAIL),
            alias,
            descriptor,
        ));
    }
    let mut wrapper = Box::new(unsafe { *original });
    wrapper.xClose = Some(secure_close);
    if kind == ArtifactKind::Main && wrapper.xShmMap.is_some() {
        wrapper.xShmMap = Some(super::secure_vfs_shm::map);
    }
    if kind == ArtifactKind::Main && wrapper.xShmUnmap.is_some() {
        wrapper.xShmUnmap = Some(super::secure_vfs_shm::unmap);
    }
    if descriptor.is_retained() && wrapper.xFileControl.is_some() {
        wrapper.xFileControl = Some(super::secure_vfs_control::file_control);
    }
    let wrapper_pointer: *const ffi::sqlite3_io_methods = &*wrapper;
    let mut states = lock_states();
    if states.contains_key(&(file as usize)) {
        return Err(RejectedInstall::new(
            file,
            anyhow::anyhow!(DUPLICATE_STATE_DETAIL),
            alias,
            descriptor,
        ));
    }
    let artifact_path = descriptor.path().to_path_buf();
    states.insert(
        file as usize,
        FileState {
            alias,
            artifact_path,
            descriptor,
            shm_descriptor: None,
            original_methods: original as usize,
            _wrapper_methods: wrapper,
        },
    );
    drop(states);
    unsafe { (*file).pMethods = wrapper_pointer };
    Ok(())
}
