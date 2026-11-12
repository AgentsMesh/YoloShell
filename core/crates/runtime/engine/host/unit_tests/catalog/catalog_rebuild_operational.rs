use crate::host::tests::*;
use super::*;

const OPERATIONAL_BEGIN_SQL: &str = "BEGIN IMMEDIATE";

#[test]
fn busy_catalog_fails_without_quarantining_a_healthy_database() {
    let directory = tempdir().unwrap();
    let initialized = Host::open(HostConfig::new(directory.path())).unwrap();
    let catalog_path = initialized.paths.catalog();
    drop(initialized);
    let locker = rusqlite::Connection::open(&catalog_path).unwrap();
    locker.execute_batch(OPERATIONAL_BEGIN_SQL).unwrap();

    let error = Host::open(HostConfig::new(directory.path())).err().unwrap();

    assert!(error.chain().any(|cause| {
        cause
            .downcast_ref::<rusqlite::Error>()
            .and_then(rusqlite::Error::sqlite_error_code)
            == Some(rusqlite::ffi::ErrorCode::DatabaseBusy)
    }));
    assert_eq!(catalog_quarantine_count(directory.path()), 0);
    assert!(catalog_path.is_file());
}
