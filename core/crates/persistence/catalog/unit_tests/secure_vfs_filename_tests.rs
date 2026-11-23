use super::*;
use std::cell::{Cell, RefCell};
use std::ffi::CStr;
use std::os::unix::ffi::OsStringExt;

const URI_PARAMETER_NAME: &[u8] = b"mode\0";
const IO_METHODS_VERSION: libc::c_int = 1;
const EMPTY_SQLITE_BYTES: &[u8] = b"";
const CATALOG_FILE: &str = "catalog.db";
const SECOND_CATALOG_FILE: &str = "second-catalog.db";

thread_local! {
    static CLOSE_FILENAME: Cell<ffi::sqlite3_filename> = const { Cell::new(std::ptr::null()) };
    static CLOSE_EXPECTED: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static CLOSE_ALIAS: RefCell<Option<std::path::PathBuf>> = const { RefCell::new(None) };
    static CLOSE_OBSERVED: Cell<bool> = const { Cell::new(false) };
}

#[test]
fn official_filename_layout_maps_every_delegated_artifact_to_its_database() {
    let directory = tempfile::tempdir().unwrap();
    let main = directory.path().join(CATALOG_FILE);
    let expected_journal = append_suffix(&main, SQLITE_JOURNAL_SUFFIX);
    let expected_wal = append_suffix(&main, SQLITE_WAL_SUFFIX);

    for kind in [ArtifactKind::Main, ArtifactKind::Journal, ArtifactKind::Wal] {
        let filename = SqliteFilename::create(&main, kind).unwrap();
        let database = unsafe { ffi::sqlite3_filename_database(filename.opened()) };
        let journal = unsafe { ffi::sqlite3_filename_journal(database) };
        let wal = unsafe { ffi::sqlite3_filename_wal(database) };

        assert_eq!(path(database), main);
        assert_eq!(path(journal), expected_journal);
        assert_eq!(path(wal), expected_wal);
        assert!(
            unsafe {
                ffi::sqlite3_uri_parameter(filename.opened(), URI_PARAMETER_NAME.as_ptr().cast())
            }
            .is_null()
        );
    }
}

#[test]
fn shm_cannot_be_delegated_as_a_standalone_sqlite_filename() {
    let directory = tempfile::tempdir().unwrap();
    let error = SqliteFilename::create(&directory.path().join(CATALOG_FILE), ArtifactKind::Shm)
        .err()
        .unwrap();
    assert!(error.to_string().contains(FILENAME_KIND_DETAIL));
}

#[test]
fn failed_state_install_returns_the_filename_owner_to_the_caller() {
    let directory = tempfile::tempdir().unwrap();
    let main = directory.path().join(CATALOG_FILE);
    let second = directory.path().join(SECOND_CATALOG_FILE);
    std::fs::write(&main, EMPTY_SQLITE_BYTES).unwrap();
    std::fs::write(&second, EMPTY_SQLITE_BYTES).unwrap();
    let first_descriptor = descriptor(&main);
    let first_alias =
        super::super::secure_vfs_alias::SecureAlias::create(&first_descriptor, ArtifactKind::Main)
            .unwrap();
    let mut methods: ffi::sqlite3_io_methods = unsafe { std::mem::zeroed() };
    methods.iVersion = IO_METHODS_VERSION;
    methods.xClose = Some(observing_close);
    let mut file = ffi::sqlite3_file { pMethods: &methods };
    let installed = unsafe {
        super::super::secure_vfs_state::install(
            &mut file,
            first_alias,
            first_descriptor,
            ArtifactKind::Main,
        )
    };
    assert!(installed.is_ok());
    let second_descriptor = descriptor(&second);
    let second_alias =
        super::super::secure_vfs_alias::SecureAlias::create(&second_descriptor, ArtifactKind::Main)
            .unwrap();
    let rejected_alias = second_alias.directory().to_path_buf();
    prepare_close_observation(
        second_alias.sqlite_name(),
        second_alias.opened_path(),
        &rejected_alias,
    );

    let rejected = unsafe {
        super::super::secure_vfs_state::install(
            &mut file,
            second_alias,
            second_descriptor,
            ArtifactKind::Main,
        )
    }
    .unwrap_err();
    let error = rejected.close();

    assert!(
        error
            .to_string()
            .contains(super::super::secure_vfs_state::DUPLICATE_STATE_DETAIL)
    );
    assert!(CLOSE_OBSERVED.with(Cell::get));
    assert!(!rejected_alias.exists());
}

fn path(raw: ffi::sqlite3_filename) -> std::path::PathBuf {
    let bytes = unsafe { CStr::from_ptr(raw) }.to_bytes();
    std::path::PathBuf::from(std::ffi::OsString::from_vec(bytes.to_vec()))
}

fn descriptor(path: &std::path::Path) -> super::super::secure_vfs_file::SecureDescriptor {
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    let flags = ffi::SQLITE_OPEN_MAIN_DB | ffi::SQLITE_OPEN_READWRITE;
    super::super::secure_vfs_file::SecureDescriptor::open(&name, flags).unwrap()
}

fn prepare_close_observation(
    filename: ffi::sqlite3_filename,
    expected: &std::path::Path,
    alias: &std::path::Path,
) {
    CLOSE_FILENAME.with(|slot| slot.set(filename));
    CLOSE_EXPECTED.with(|slot| *slot.borrow_mut() = expected.as_os_str().as_bytes().to_vec());
    CLOSE_ALIAS.with(|slot| *slot.borrow_mut() = Some(alias.to_path_buf()));
    CLOSE_OBSERVED.with(|slot| slot.set(false));
}

unsafe extern "C" fn observing_close(_: *mut ffi::sqlite3_file) -> libc::c_int {
    let filename = CLOSE_FILENAME.with(Cell::get);
    let database = unsafe { ffi::sqlite3_filename_database(filename) };
    let observed = !database.is_null()
        && CLOSE_EXPECTED.with(|expected| {
            unsafe { CStr::from_ptr(database) }.to_bytes() == expected.borrow().as_slice()
        })
        && CLOSE_ALIAS.with(|alias| alias.borrow().as_ref().is_some_and(|path| path.is_dir()));
    CLOSE_OBSERVED.with(|slot| slot.set(observed));
    ffi::SQLITE_OK
}
