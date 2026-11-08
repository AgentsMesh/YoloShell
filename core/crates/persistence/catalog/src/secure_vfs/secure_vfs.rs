use super::secure_vfs_alias::SecureAlias;
use super::secure_vfs_file::{ArtifactKind, SecureDescriptor};
use super::secure_vfs_identity::is_unsafe_artifact;
use anyhow::Result;
use rusqlite::{Connection, OpenFlags, ffi};
use std::cell::RefCell;
use std::ffi::CStr;
use std::path::Path;
use std::sync::OnceLock;

const BASE_VFS_NAME: &[u8] = b"unix\0";
const SECURE_VFS_NAME: &str = "yolo-secure";
const SECURE_VFS_NAME_BYTES: &[u8] = b"yolo-secure\0";

static VFS_REGISTRATION: OnceLock<std::result::Result<(), String>> = OnceLock::new();

thread_local! {
    static LAST_VIOLATION: RefCell<Option<String>> = const { RefCell::new(None) };
    static LAST_FAILURE: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub(super) fn open(path: &Path, flags: OpenFlags) -> Result<Connection> {
    register()?;
    take_violation();
    LAST_FAILURE.with(|failure| failure.borrow_mut().take());
    let result = Connection::open_with_flags_and_vfs(path, flags, SECURE_VFS_NAME);
    sqlite_result(result)
}

pub(super) fn sqlite_result<T>(result: rusqlite::Result<T>) -> Result<T> {
    let failure = LAST_FAILURE.with(|failure| failure.borrow_mut().take());
    match (result, take_violation()) {
        (_, Some(message)) => Err(super::rebuild_reason::unsafe_artifact(message)),
        (Ok(value), None) => Ok(value),
        (Err(error), None) => match failure {
            Some(context) => Err(anyhow::Error::from(error).context(context)),
            None => Err(error.into()),
        },
    }
}

pub(super) fn take_violation() -> Option<String> {
    LAST_VIOLATION.with(|violation| violation.borrow_mut().take())
}

pub(super) fn callback_error(error: anyhow::Error) -> libc::c_int {
    if is_unsafe_artifact(&error) {
        record_violation(error.to_string());
    } else {
        LAST_FAILURE.with(|failure| *failure.borrow_mut() = Some(format!("{error:#}")));
    }
    ffi::SQLITE_CANTOPEN
}

pub(super) fn record_violation(message: String) {
    LAST_VIOLATION.with(|violation| *violation.borrow_mut() = Some(message));
}

fn register() -> Result<()> {
    VFS_REGISTRATION
        .get_or_init(register_inner)
        .as_ref()
        .map(|_| ())
        .map_err(|message| anyhow::anyhow!(message.clone()))
}

fn register_inner() -> std::result::Result<(), String> {
    let base_name = CStr::from_bytes_with_nul(BASE_VFS_NAME).map_err(|error| error.to_string())?;
    let base = unsafe { ffi::sqlite3_vfs_find(base_name.as_ptr()) };
    if base.is_null() {
        return Err("SQLite unix VFS is unavailable".to_owned());
    }
    let mut wrapper = Box::new(unsafe { *base });
    wrapper.pNext = std::ptr::null_mut();
    wrapper.zName = SECURE_VFS_NAME_BYTES.as_ptr().cast();
    wrapper.pAppData = base.cast();
    wrapper.xOpen = Some(secure_open);
    wrapper.xDelete = Some(super::secure_vfs_path::delete);
    wrapper.xAccess = Some(super::secure_vfs_path::access);
    wrapper.xFullPathname = Some(super::secure_vfs_path::full_pathname);
    let wrapper = Box::into_raw(wrapper);
    let code = unsafe { ffi::sqlite3_vfs_register(wrapper, 0) };
    if code == ffi::SQLITE_OK {
        Ok(())
    } else {
        unsafe { drop(Box::from_raw(wrapper)) };
        Err(format!(
            "register secure SQLite VFS failed with code {code}"
        ))
    }
}

pub(super) unsafe extern "C" fn secure_open(
    wrapper: *mut ffi::sqlite3_vfs,
    name: ffi::sqlite3_filename,
    file: *mut ffi::sqlite3_file,
    flags: libc::c_int,
    output_flags: *mut libc::c_int,
) -> libc::c_int {
    let base = unsafe { (*wrapper).pAppData.cast::<ffi::sqlite3_vfs>() };
    let Some(base_open) = (unsafe { (*base).xOpen }) else {
        return ffi::SQLITE_CANTOPEN;
    };
    let Some(kind) = ArtifactKind::from_open_flags(flags) else {
        return unsafe { base_open(base, name, file, flags, output_flags) };
    };
    if name.is_null() {
        return ffi::SQLITE_CANTOPEN;
    }
    let name = unsafe { CStr::from_ptr(name) };
    let descriptor = match SecureDescriptor::open(name, flags) {
        Ok(descriptor) => descriptor,
        Err(error) => return callback_error(error),
    };
    run_test_hook(kind, descriptor.path());
    let alias = match SecureAlias::create(&descriptor, kind) {
        Ok(alias) => alias,
        Err(error) => return callback_error(error),
    };
    if let Err(error) = alias.seal() {
        return callback_error(error);
    }
    run_alias_test_hook(kind, alias.opened_path());
    if let Err(error) = descriptor.verify_alias(alias.opened_path()) {
        let _ = alias.unseal();
        return callback_error(error);
    }
    let delegated_flags = flags & !ffi::SQLITE_OPEN_EXCLUSIVE;
    let code = unsafe {
        base_open(
            base,
            alias.sqlite_name(),
            file,
            delegated_flags,
            output_flags,
        )
    };
    if code != ffi::SQLITE_OK {
        let _ = alias.unseal();
        alias.discard_open_link();
        return code;
    }
    if let Err(error) = descriptor.verify_alias(alias.opened_path()) {
        unsafe { close_after_rejection(file) };
        let _ = alias.unseal();
        alias.discard_open_link();
        return callback_error(error);
    }
    if let Err(error) = alias.unseal() {
        unsafe { close_after_rejection(file) };
        return callback_error(error);
    }
    if let Err(error) = alias.release_open_link(&descriptor) {
        unsafe { close_after_rejection(file) };
        return callback_error(error);
    }
    if let Err(error) = descriptor.verify_current() {
        unsafe { close_after_rejection(file) };
        return callback_error(error);
    }
    if let Err(rejected) =
        unsafe { super::secure_vfs_state::install(file, alias, descriptor, kind) }
    {
        return callback_error(rejected.close());
    }
    code
}

pub(super) unsafe fn close_after_rejection(file: *mut ffi::sqlite3_file) {
    let methods = unsafe { (*file).pMethods };
    if !methods.is_null()
        && let Some(close) = unsafe { (*methods).xClose }
    {
        unsafe { close(file) };
    }
}

pub(super) fn run_test_hook(kind: ArtifactKind, path: &Path) {
    #[cfg(all(test, feature = "yolo-unit-tests"))]
    super::secure_vfs_tests::run(kind, path);
    #[cfg(not(all(test, feature = "yolo-unit-tests")))]
    let _ = (kind, path);
}

pub(super) fn run_alias_test_hook(kind: ArtifactKind, path: &Path) {
    #[cfg(all(test, feature = "yolo-unit-tests"))]
    super::secure_vfs_tests::run_alias(kind, path);
    #[cfg(not(all(test, feature = "yolo-unit-tests")))]
    let _ = (kind, path);
}
