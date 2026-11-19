use super::retained_test_fixture::{
    PRIVATE_FILE_MODE, REPLACEMENT_SENTINEL, RetainedFixture, SENTINEL_BYTES, WAL_SUFFIX,
};
use rusqlite::ffi;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const DISPLACED_WAL_NAME: &str = "opened-wal";
const DISPLACED_SYMLINK_WAL_NAME: &str = "symlink-wal";
const INVALID_ACCESS_REQUEST: libc::c_int = libc::c_int::MAX;
const UNSAFE_FILE_MODE: u32 = 0o000;

#[test]
fn xdelete_rejects_a_stable_wal_replacement_without_deleting_it() {
    let fixture = RetainedFixture::new();
    let (catalog, _) =
        super::Catalog::open_rebuildable_retained(fixture.entry(), || Ok(Vec::new())).unwrap();
    let expected_wal = fixture.sidecar(WAL_SUFFIX);
    let displaced_wal = fixture.home.join(DISPLACED_WAL_NAME);
    let hook_ran = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&hook_ran);
    super::retained_delete::set_test_hook(move |path| {
        anyhow::ensure!(path == expected_wal, "unexpected retained delete target");
        std::fs::rename(path, &displaced_wal)?;
        std::fs::write(path, SENTINEL_BYTES)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(PRIVATE_FILE_MODE))?;
        observed.store(true, Ordering::Release);
        Ok(())
    });

    drop(catalog);

    assert!(hook_ran.load(Ordering::Acquire));
    assert_eq!(
        std::fs::read(fixture.sidecar(WAL_SUFFIX)).unwrap(),
        SENTINEL_BYTES
    );
}

#[test]
fn retained_delete_succeeds_after_home_replacement_and_syncs_the_original_parent() {
    let fixture = RetainedFixture::new();
    let wal = fixture.sidecar(WAL_SUFFIX);
    fixture.create_private(&wal, b"trusted WAL");
    let namespace =
        super::retained_namespace::RetainedCatalogNamespace::register(fixture.entry()).unwrap();
    namespace.snapshot().unwrap();
    fixture.replace_home();
    let artifact = super::retained_namespace::resolve(&wal).unwrap();

    super::retained_delete::remove_artifact(&artifact).unwrap();

    assert!(!fixture.displaced.join(wal.file_name().unwrap()).exists());
    fixture.assert_replacement_untouched();
}

#[test]
fn retained_access_honors_every_sqlite_mode_and_rejects_identity_changes() {
    let fixture = RetainedFixture::new();
    fixture.create_private(&fixture.catalog, b"trusted catalog");
    let namespace =
        super::retained_namespace::RetainedCatalogNamespace::register(fixture.entry()).unwrap();
    namespace.snapshot().unwrap();
    let artifact = super::retained_namespace::resolve(&fixture.catalog).unwrap();
    for request in [
        ffi::SQLITE_ACCESS_EXISTS,
        ffi::SQLITE_ACCESS_READ,
        ffi::SQLITE_ACCESS_READWRITE,
    ] {
        assert!(super::retained_access::access(&artifact, request).unwrap());
    }
    assert!(super::retained_access::access(&artifact, INVALID_ACCESS_REQUEST).is_err());
    let original = fixture.home.join("original-catalog");
    std::fs::rename(&fixture.catalog, original).unwrap();
    fixture.create_private(&fixture.catalog, SENTINEL_BYTES);

    assert!(super::retained_access::access(&artifact, ffi::SQLITE_ACCESS_EXISTS).is_err());
}

#[test]
fn missing_unbound_sidecar_reports_not_accessible() {
    let fixture = RetainedFixture::new();
    let namespace =
        super::retained_namespace::RetainedCatalogNamespace::register(fixture.entry()).unwrap();
    namespace.snapshot().unwrap();
    let wal = super::retained_namespace::resolve(&fixture.sidecar(WAL_SUFFIX)).unwrap();

    assert!(!super::retained_access::access(&wal, ffi::SQLITE_ACCESS_EXISTS).unwrap());
    assert!(!fixture.home.join(REPLACEMENT_SENTINEL).exists());
}

#[test]
fn access_rejects_an_artifact_without_private_permissions() {
    let fixture = RetainedFixture::new();
    fixture.create_private(&fixture.catalog, b"trusted catalog");
    let namespace =
        super::retained_namespace::RetainedCatalogNamespace::register(fixture.entry()).unwrap();
    namespace.snapshot().unwrap();
    let artifact = super::retained_namespace::resolve(&fixture.catalog).unwrap();
    std::fs::set_permissions(
        &fixture.catalog,
        std::fs::Permissions::from_mode(UNSAFE_FILE_MODE),
    )
    .unwrap();

    assert!(super::retained_access::access(&artifact, ffi::SQLITE_ACCESS_READ).is_err());
    assert!(super::retained_access::access(&artifact, ffi::SQLITE_ACCESS_READWRITE).is_err());
}

#[test]
fn fsync_failure_reports_error_and_a_retry_converges_after_the_unlink() {
    let fixture = RetainedFixture::new();
    let wal = fixture.sidecar(WAL_SUFFIX);
    fixture.create_private(&wal, b"trusted WAL");
    let namespace =
        super::retained_namespace::RetainedCatalogNamespace::register(fixture.entry()).unwrap();
    namespace.snapshot().unwrap();
    let artifact = super::retained_namespace::resolve(&wal).unwrap();
    super::retained_delete::set_sync_test_hook(|| anyhow::bail!("injected parent fsync failure"));

    assert!(super::retained_delete::remove_artifact(&artifact).is_err());
    assert!(!wal.exists());
    super::retained_delete::remove_artifact(&artifact).unwrap();
    assert!(artifact.expected().is_none());
}

#[test]
fn unbound_delete_accepts_only_a_missing_sidecar() {
    let fixture = RetainedFixture::new();
    let namespace =
        super::retained_namespace::RetainedCatalogNamespace::register(fixture.entry()).unwrap();
    namespace.snapshot().unwrap();
    let wal_path = fixture.sidecar(WAL_SUFFIX);
    let artifact = super::retained_namespace::resolve(&wal_path).unwrap();

    super::retained_delete::remove_artifact(&artifact).unwrap();
    fixture.create_private(&wal_path, SENTINEL_BYTES);

    assert!(super::retained_delete::remove_artifact(&artifact).is_err());
    assert_eq!(std::fs::read(wal_path).unwrap(), SENTINEL_BYTES);
}

#[test]
fn isolated_symlink_replacement_is_restored_and_rejected() {
    let fixture = RetainedFixture::new();
    let wal = fixture.sidecar(WAL_SUFFIX);
    fixture.create_private(&wal, SENTINEL_BYTES);
    let namespace =
        super::retained_namespace::RetainedCatalogNamespace::register(fixture.entry()).unwrap();
    namespace.snapshot().unwrap();
    let artifact = super::retained_namespace::resolve(&wal).unwrap();
    let displaced = fixture.home.join(DISPLACED_SYMLINK_WAL_NAME);
    super::retained_delete::set_test_hook(move |path| {
        std::fs::rename(path, &displaced)?;
        std::os::unix::fs::symlink(&displaced, path)?;
        Ok(())
    });

    assert!(super::retained_delete::remove_artifact(&artifact).is_err());
    assert!(
        std::fs::symlink_metadata(&wal)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}
