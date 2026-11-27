use crate::host::tests::*;
const DURABLE_MATRIX_MISSING_NAME: &str = "missing-durable-entry";
const DURABLE_MATRIX_FILE_NAME: &str = "durable-entry";
const DURABLE_MATRIX_SYMLINK_NAME: &str = "durable-entry-link";
const DURABLE_MATRIX_NUL_NAME: &[u8] = b"invalid\0durable";
const DURABLE_MATRIX_EMPTY_LIMIT: usize = 0;
const DURABLE_MATRIX_WRONG_READ_LENGTH: u64 = 0;
const DURABLE_MATRIX_UNSAFE_FILE: &str = "unsafe-durable-entry";
const DURABLE_MATRIX_UNSAFE_MODE: u32 = 0o640;

#[test]
fn durable_directory_and_identity_boundaries_fail_closed() {
    use std::io::Write;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{PermissionsExt, symlink};
    let directory = tempdir().unwrap();
    let missing = directory.path().join(DURABLE_MATRIX_MISSING_NAME);
    assert!(catalog_durable_dir::DurableDirectory::open(&missing).is_err());
    crate::paths::directory::PrivateDirectory::ensure(directory.path()).unwrap();
    let durable = catalog_durable_dir::DurableDirectory::open(directory.path()).unwrap();
    let missing_name = missing.file_name().unwrap();
    assert!(durable.open_directory(missing_name).unwrap().is_none());
    assert!(durable.open_file(missing_name).unwrap().is_none());
    let invalid = std::ffi::OsStr::from_bytes(DURABLE_MATRIX_NUL_NAME);
    assert!(durable.open_directory(invalid).is_err());
    assert!(durable.open_file(invalid).is_err());
    let file = directory.path().join(DURABLE_MATRIX_FILE_NAME);
    write_private_bytes(&file, DURABLE_MATRIX_MINIMAL_JSON);
    assert!(durable.open_directory(file.file_name().unwrap()).is_err());
    let link = directory.path().join(DURABLE_MATRIX_SYMLINK_NAME);
    symlink(&file, &link).unwrap();
    assert!(durable.open_file(link.file_name().unwrap()).is_err());
    assert!(durable.entries_bounded(DURABLE_MATRIX_EMPTY_LIMIT).is_err());
    let unsafe_file = directory.path().join(DURABLE_MATRIX_UNSAFE_FILE);
    write_private_bytes(&unsafe_file, DURABLE_MATRIX_MINIMAL_JSON);
    std::fs::set_permissions(
        &unsafe_file,
        std::fs::Permissions::from_mode(DURABLE_MATRIX_UNSAFE_MODE),
    )
    .unwrap();
    assert!(
        catalog_durable_identity::DurableIdentity::inspect(
            &std::fs::metadata(&unsafe_file).unwrap(),
            &unsafe_file,
        )
        .is_err()
    );

    let opened = std::fs::File::open(&file).unwrap();
    let identity =
        catalog_durable_identity::DurableIdentity::inspect(&opened.metadata().unwrap(), &file)
            .unwrap();
    assert!(identity.same_version(&identity));
    assert!(
        identity
            .verify(
                &opened.metadata().unwrap(),
                DURABLE_MATRIX_WRONG_READ_LENGTH,
                &file,
            )
            .is_err()
    );
    std::fs::OpenOptions::new()
        .append(true)
        .open(&file)
        .unwrap()
        .write_all(DURABLE_MATRIX_GROWTH_BYTES)
        .unwrap();
    let length_changed = catalog_durable_identity::DurableIdentity::inspect(
        &std::fs::metadata(&file).unwrap(),
        &file,
    )
    .unwrap();
    assert!(!identity.same_version(&length_changed));
    let other = directory.path().join(DURABLE_MATRIX_SAVED_NAME);
    write_private_bytes(&other, DURABLE_MATRIX_MINIMAL_JSON);
    let other_file = std::fs::OpenOptions::new()
        .write(true)
        .open(&other)
        .unwrap();
    let other_identity =
        catalog_durable_identity::DurableIdentity::inspect(&other_file.metadata().unwrap(), &other)
            .unwrap();
    assert!(!identity.same_version(&other_identity));
    other_file
        .set_times(std::fs::FileTimes::new().set_modified(std::time::SystemTime::UNIX_EPOCH))
        .unwrap();
    let time_changed =
        catalog_durable_identity::DurableIdentity::inspect(&other_file.metadata().unwrap(), &other)
            .unwrap();
    assert!(!other_identity.same_version(&time_changed));
}
