use crate::host::tests::*;
const DURABLE_WAVE5_HARD_LINK: &str = "durable-hard-link";
const DURABLE_WAVE5_OVERSIZE: &str = "durable-oversize";
const DURABLE_WAVE5_UNSAFE_MODE: &str = "durable-unsafe-mode";
const DURABLE_WAVE5_OVERSIZE_INCREMENT: u64 = 1;
const DURABLE_WAVE5_UNSAFE_FILE_MODE: u32 = 0o640;

#[test]
fn durable_identity_rejects_each_hermetic_metadata_violation() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir().unwrap();
    assert!(
        catalog_durable_identity::DurableIdentity::inspect(
            &std::fs::metadata(directory.path()).unwrap(),
            directory.path(),
        )
        .is_err()
    );

    let hard_link_source = directory.path().join(DURABLE_MATRIX_FILE_NAME);
    let hard_link = directory.path().join(DURABLE_WAVE5_HARD_LINK);
    write_private_bytes(&hard_link_source, DURABLE_MATRIX_MINIMAL_JSON);
    std::fs::hard_link(&hard_link_source, &hard_link).unwrap();
    assert!(
        catalog_durable_identity::DurableIdentity::inspect(
            &std::fs::metadata(&hard_link_source).unwrap(),
            &hard_link_source,
        )
        .is_err()
    );

    let oversized = directory.path().join(DURABLE_WAVE5_OVERSIZE);
    write_private_bytes(&oversized, DURABLE_MATRIX_MINIMAL_JSON);
    std::fs::OpenOptions::new()
        .write(true)
        .open(&oversized)
        .unwrap()
        .set_len(
            catalog_durable_file::MAX_DURABLE_JSON_BYTES
                .saturating_add(DURABLE_WAVE5_OVERSIZE_INCREMENT),
        )
        .unwrap();
    assert!(
        catalog_durable_identity::DurableIdentity::inspect(
            &std::fs::metadata(&oversized).unwrap(),
            &oversized,
        )
        .is_err()
    );

    let unsafe_mode = directory.path().join(DURABLE_WAVE5_UNSAFE_MODE);
    write_private_bytes(&unsafe_mode, DURABLE_MATRIX_MINIMAL_JSON);
    std::fs::set_permissions(
        &unsafe_mode,
        std::fs::Permissions::from_mode(DURABLE_WAVE5_UNSAFE_FILE_MODE),
    )
    .unwrap();
    assert!(
        catalog_durable_identity::DurableIdentity::inspect(
            &std::fs::metadata(&unsafe_mode).unwrap(),
            &unsafe_mode,
        )
        .is_err()
    );
}

#[test]
fn durable_directory_lists_and_opens_only_owned_entries() {
    let directory = tempdir().unwrap();
    crate::paths::directory::PrivateDirectory::ensure(directory.path()).unwrap();
    let nested = directory.path().join(DURABLE_MATRIX_SAVED_NAME);
    crate::paths::directory::PrivateDirectory::ensure(&nested).unwrap();
    let file = directory.path().join(DURABLE_MATRIX_FILE_NAME);
    write_private_bytes(&file, DURABLE_MATRIX_MINIMAL_JSON);
    let durable = catalog_durable_dir::DurableDirectory::open(directory.path()).unwrap();

    assert!(
        durable
            .open_directory(nested.file_name().unwrap())
            .unwrap()
            .is_some()
    );
    assert!(
        durable
            .open_file(file.file_name().unwrap())
            .unwrap()
            .is_some()
    );
    let entries = durable.entries().unwrap();
    assert!(entries.contains(&nested.file_name().unwrap().to_owned()));
    assert!(entries.contains(&file.file_name().unwrap().to_owned()));
}
