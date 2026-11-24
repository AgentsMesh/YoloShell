use super::*;

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn operational_sqlite_errors_are_not_rebuildable_schema_errors() {
    let codes = [
        rusqlite::ffi::SQLITE_NOMEM,
        rusqlite::ffi::SQLITE_ABORT,
        rusqlite::ffi::SQLITE_MISUSE,
        rusqlite::ffi::SQLITE_IOERR,
        rusqlite::ffi::SQLITE_BUSY,
        rusqlite::ffi::SQLITE_LOCKED,
        rusqlite::ffi::SQLITE_FULL,
        rusqlite::ffi::SQLITE_READONLY,
        rusqlite::ffi::SQLITE_PERM,
    ];
    for code in codes {
        let error = rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), None);
        let classified = schema_result::<()>(Err(error)).unwrap_err();
        assert!(!is_rebuildable(&classified), "code {code} was rebuildable");
    }
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn explicit_schema_context_is_rebuildable() {
    let error = rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ERROR),
        None,
    );
    let classified = schema_result::<()>(Err(error)).unwrap_err();
    assert!(is_rebuildable(&classified));
}
