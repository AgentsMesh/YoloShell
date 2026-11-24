use super::Catalog;
use crate::paths::Paths;
use crate::paths::directory::PrivateDirectory;
use crate::paths::namespace_lease::NamespaceLease;
use crate::paths::private_file::PrivateFileEntry;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const HOME_NAME: &str = "home";
const DISPLACED_NAME: &str = "displaced-home";
const NAMESPACE_LABEL: &str = "catalog-test-home";
const SENTINEL_NAME: &str = "replacement-sentinel";
const SENTINEL_CONTENT: &[u8] = b"replacement must remain untouched";
const MAIN_DATABASE_NAME: &[u8] = b"main\0";
const WAL_CHECKPOINT_SQL: &str = "PRAGMA wal_checkpoint(TRUNCATE)";

struct CatalogRoot {
    _root: tempfile::TempDir,
    home: PathBuf,
    displaced: PathBuf,
    catalog: PathBuf,
    lease: NamespaceLease,
}

impl CatalogRoot {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join(HOME_NAME);
        let displaced = root.path().join(DISPLACED_NAME);
        PrivateDirectory::ensure(&home).unwrap();
        let lease = NamespaceLease::capture(&[(NAMESPACE_LABEL, &home)]).unwrap();
        let catalog = Paths::new(&home).catalog();
        Self {
            _root: root,
            home,
            displaced,
            catalog,
            lease,
        }
    }

    fn entry(&self) -> PrivateFileEntry {
        self.lease.file(&self.catalog).unwrap()
    }

    fn replace_home(&self) {
        replace_home(&self.home, &self.displaced).unwrap();
    }

    fn assert_replacement_untouched(&self) {
        let entries = std::fs::read_dir(&self.home)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert_eq!(entries, [SENTINEL_NAME]);
        assert_eq!(
            std::fs::read(self.home.join(SENTINEL_NAME)).unwrap(),
            SENTINEL_CONTENT
        );
    }

    fn displaced_catalog(&self) -> PathBuf {
        self.displaced.join(self.catalog.file_name().unwrap())
    }
}

#[test]
fn opening_during_home_replacement_never_writes_the_replacement() {
    for kind in [
        super::secure_vfs_file::ArtifactKind::Main,
        super::secure_vfs_file::ArtifactKind::Wal,
        super::secure_vfs_file::ArtifactKind::Shm,
    ] {
        assert_open_replacement_is_safe(kind);
    }
}

fn assert_open_replacement_is_safe(kind: super::secure_vfs_file::ArtifactKind) {
    let fixture = CatalogRoot::new();
    let home = fixture.home.clone();
    let displaced = fixture.displaced.clone();
    super::secure_vfs_tests::set(kind, move |_| replace_home(&home, &displaced));

    let result = Catalog::open_rebuildable_retained(fixture.entry(), || Ok(Vec::new()));

    drop(result);
    assert!(fixture.displaced_catalog().exists());
    fixture.assert_replacement_untouched();
    assert!(!super::retained_namespace::is_registered(&fixture.catalog));
}

#[test]
fn open_connection_keeps_writes_and_checkpoint_out_of_replacement() {
    let fixture = CatalogRoot::new();
    let opened_alias = Arc::new(Mutex::new(None));
    let observed_alias = Arc::clone(&opened_alias);
    super::secure_vfs_tests::set_alias(super::secure_vfs_file::ArtifactKind::Main, move |path| {
        *observed_alias.lock().unwrap() = Some(path.to_path_buf());
        Ok(())
    });
    let (mut catalog, rebuilt) =
        Catalog::open_rebuildable_retained(fixture.entry(), || Ok(Vec::new())).unwrap();
    assert!(!rebuilt);
    let opened_alias = opened_alias.lock().unwrap().take().unwrap();
    assert!(!opened_alias.exists());
    let alias_directory = opened_alias.parent().unwrap().to_path_buf();
    let previous_generation = catalog.controller_generation().unwrap();
    fixture.replace_home();
    assert_file_control_contract(&catalog);

    let next_generation = catalog.bump_controller_generation().unwrap();
    catalog
        .connection
        .execute_batch(WAL_CHECKPOINT_SQL)
        .unwrap();
    drop(catalog);

    assert!(next_generation > previous_generation);
    assert!(!alias_directory.exists());
    fixture.assert_replacement_untouched();
    let reopened = Catalog::open(&fixture.displaced_catalog()).unwrap();
    assert_eq!(reopened.controller_generation().unwrap(), next_generation);
}

#[test]
fn same_slot_catalogs_unregister_only_after_the_last_generation_drops() {
    let fixture = CatalogRoot::new();
    let (first, rebuilt) =
        Catalog::open_rebuildable_retained(fixture.entry(), || Ok(Vec::new())).unwrap();
    assert!(!rebuilt);
    let (mut overlapping, rebuilt) =
        Catalog::open_rebuildable_retained(fixture.entry(), || Ok(Vec::new())).unwrap();
    assert!(!rebuilt);
    let public_open = Catalog::open(&fixture.catalog).unwrap();
    assert!(super::retained_namespace::is_registered(&fixture.catalog));
    drop(first);
    assert!(super::retained_namespace::is_registered(&fixture.catalog));
    overlapping.bump_controller_generation().unwrap();
    drop(overlapping);
    assert!(super::retained_namespace::is_registered(&fixture.catalog));
    drop(public_open);
    assert!(!super::retained_namespace::is_registered(&fixture.catalog));
}

fn replace_home(home: &Path, displaced: &Path) -> anyhow::Result<()> {
    std::fs::rename(home, displaced)?;
    PrivateDirectory::ensure(home)?;
    std::fs::write(home.join(SENTINEL_NAME), SENTINEL_CONTENT)?;
    Ok(())
}

fn assert_file_control_contract(catalog: &Catalog) {
    let mut moved = libc::c_int::MAX;
    let moved_code = unsafe {
        rusqlite::ffi::sqlite3_file_control(
            catalog.connection.handle(),
            MAIN_DATABASE_NAME.as_ptr().cast(),
            rusqlite::ffi::SQLITE_FCNTL_HAS_MOVED,
            (&raw mut moved).cast(),
        )
    };
    assert_eq!(moved_code, rusqlite::ffi::SQLITE_OK);
    assert_eq!(moved, super::secure_vfs_control::FILE_NOT_MOVED);

    let delegated_code = unsafe {
        rusqlite::ffi::sqlite3_file_control(
            catalog.connection.handle(),
            MAIN_DATABASE_NAME.as_ptr().cast(),
            libc::c_int::MAX,
            std::ptr::null_mut(),
        )
    };
    assert_eq!(delegated_code, rusqlite::ffi::SQLITE_NOTFOUND);
}
