use rusqlite::ffi;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;

const CATALOG_FILE: &str = "catalog.sqlite";
const MISSING_FILE: &str = "missing.sqlite";
const UNSAFE_DETAIL: &str = "injected unsafe SQLite artifact";
const EMPTY_SQLITE_BYTES: &[u8] = b"";
const EXPECTED_FINAL_LINK_COUNT: u64 = 1;

type OpenCallback = unsafe extern "C" fn(
    *mut ffi::sqlite3_vfs,
    ffi::sqlite3_filename,
    *mut ffi::sqlite3_file,
    libc::c_int,
    *mut libc::c_int,
) -> libc::c_int;

#[test]
fn secure_open_fails_closed_for_missing_handlers_and_invalid_inputs() {
    let directory = tempfile::tempdir().unwrap();
    let missing = CString::new(directory.path().join(MISSING_FILE).as_os_str().as_bytes()).unwrap();
    let cases = [
        (
            None::<OpenCallback>,
            std::ptr::null::<libc::c_char>(),
            ffi::SQLITE_OPEN_MAIN_DB,
            ffi::SQLITE_CANTOPEN,
        ),
        (
            Some(delegated_open),
            std::ptr::null(),
            ffi::SQLITE_OPEN_TEMP_DB,
            ffi::SQLITE_BUSY,
        ),
        (
            Some(delegated_open),
            std::ptr::null(),
            ffi::SQLITE_OPEN_MAIN_DB,
            ffi::SQLITE_CANTOPEN,
        ),
        (
            Some(delegated_open),
            missing.as_ptr(),
            ffi::SQLITE_OPEN_MAIN_DB | ffi::SQLITE_OPEN_READWRITE,
            ffi::SQLITE_CANTOPEN,
        ),
    ];
    for (callback, name, flags, expected) in cases {
        assert_eq!(
            invoke(callback, name, std::ptr::null_mut(), flags),
            expected
        );
    }
}

#[test]
fn secure_open_returns_the_base_failure_and_releases_its_alias() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join(CATALOG_FILE);
    std::fs::write(&artifact, EMPTY_SQLITE_BYTES).unwrap();
    let name = CString::new(artifact.as_os_str().as_bytes()).unwrap();

    let code = invoke(
        Some(rejected_open),
        name.as_ptr(),
        std::ptr::null_mut(),
        ffi::SQLITE_OPEN_MAIN_DB | ffi::SQLITE_OPEN_READWRITE,
    );

    assert_eq!(code, ffi::SQLITE_IOERR);
    assert_eq!(
        std::fs::symlink_metadata(artifact).unwrap().nlink(),
        EXPECTED_FINAL_LINK_COUNT
    );
}

#[test]
fn secure_open_rejects_a_base_file_without_io_methods() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join(CATALOG_FILE);
    std::fs::write(&artifact, EMPTY_SQLITE_BYTES).unwrap();
    let name = CString::new(artifact.as_os_str().as_bytes()).unwrap();
    let mut file = ffi::sqlite3_file {
        pMethods: std::ptr::null(),
    };

    let code = invoke(
        Some(open_without_methods),
        name.as_ptr(),
        &mut file,
        ffi::SQLITE_OPEN_MAIN_DB | ffi::SQLITE_OPEN_READWRITE,
    );

    assert_eq!(code, ffi::SQLITE_CANTOPEN);
    assert!(file.pMethods.is_null());
}

#[test]
fn callback_errors_record_only_unsafe_artifacts() {
    super::secure_vfs::take_violation();
    assert_eq!(
        super::secure_vfs::callback_error(anyhow::anyhow!(MISSING_FILE)),
        ffi::SQLITE_CANTOPEN
    );
    assert!(super::secure_vfs::take_violation().is_none());
    let unsafe_error = super::secure_vfs_identity::unsafe_artifact(UNSAFE_DETAIL.to_owned());
    assert_eq!(
        super::secure_vfs::callback_error(unsafe_error),
        ffi::SQLITE_CANTOPEN
    );
    assert_eq!(
        super::secure_vfs::take_violation().as_deref(),
        Some(UNSAFE_DETAIL)
    );
}

#[test]
fn callback_io_context_survives_sqlite_error_and_is_consumed_once() {
    super::secure_vfs::take_violation();
    super::secure_vfs::callback_error(anyhow::anyhow!(MISSING_FILE));
    let result = super::secure_vfs::sqlite_result::<()>(Err(rusqlite::Error::InvalidQuery));
    assert!(format!("{:#}", result.unwrap_err()).contains(MISSING_FILE));
    let next = super::secure_vfs::sqlite_result::<()>(Err(rusqlite::Error::InvalidQuery));
    assert!(!format!("{:#}", next.unwrap_err()).contains(MISSING_FILE));
}

fn invoke(
    callback: Option<OpenCallback>,
    name: ffi::sqlite3_filename,
    file: *mut ffi::sqlite3_file,
    flags: libc::c_int,
) -> libc::c_int {
    let registered = unsafe { ffi::sqlite3_vfs_find(std::ptr::null()) };
    assert!(!registered.is_null());
    let mut base = Box::new(unsafe { *registered });
    base.xOpen = callback;
    let mut wrapper = Box::new(unsafe { *registered });
    wrapper.pAppData = (&mut *base as *mut ffi::sqlite3_vfs).cast();
    unsafe {
        super::secure_vfs::secure_open(&mut *wrapper, name, file, flags, std::ptr::null_mut())
    }
}

unsafe extern "C" fn delegated_open(
    _: *mut ffi::sqlite3_vfs,
    _: ffi::sqlite3_filename,
    _: *mut ffi::sqlite3_file,
    _: libc::c_int,
    _: *mut libc::c_int,
) -> libc::c_int {
    ffi::SQLITE_BUSY
}

unsafe extern "C" fn rejected_open(
    _: *mut ffi::sqlite3_vfs,
    _: ffi::sqlite3_filename,
    _: *mut ffi::sqlite3_file,
    _: libc::c_int,
    _: *mut libc::c_int,
) -> libc::c_int {
    ffi::SQLITE_IOERR
}

unsafe extern "C" fn open_without_methods(
    _: *mut ffi::sqlite3_vfs,
    _: ffi::sqlite3_filename,
    _: *mut ffi::sqlite3_file,
    _: libc::c_int,
    _: *mut libc::c_int,
) -> libc::c_int {
    ffi::SQLITE_OK
}
