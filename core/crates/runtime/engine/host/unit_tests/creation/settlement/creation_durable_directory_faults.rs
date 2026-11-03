use crate::host::tests::*;
const CREATION_DIRECTORY_TEST_LABEL: &str = "creation durable test";
const CREATION_DIRECTORY_FILE_NAME: &str = "creation-parent-file";
const CREATION_DIRECTORY_PUBLIC_NAME: &str = "creation-public-parent";
const CREATION_DIRECTORY_FILE_DETAIL: &str = "parent is not a directory";
const CREATION_DIRECTORY_OWNER_DETAIL: &str = "parent is owned by another user";
const CREATION_DIRECTORY_MODE_DETAIL: &str = "parent permissions are not private";
const CREATION_DIRECTORY_ROOT: &str = "/";
const CREATION_DIRECTORY_ROOT_OWNER: u32 = 0;
const CREATION_DIRECTORY_PUBLIC_MODE: u32 = 0o755;
const CREATION_DIRECTORY_EMPTY_BYTES: &[u8] = b"";

#[test]
fn creation_directory_validation_renders_each_rejected_identity() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir().unwrap();
    let file = directory.path().join(CREATION_DIRECTORY_FILE_NAME);
    write_private_bytes(&file, CREATION_DIRECTORY_EMPTY_BYTES);
    let error = verify_private_directory(
        &file.metadata().unwrap(),
        &file,
        CREATION_DIRECTORY_TEST_LABEL,
    )
    .unwrap_err();
    assert!(error.to_string().contains(CREATION_DIRECTORY_FILE_DETAIL));

    let public = directory.path().join(CREATION_DIRECTORY_PUBLIC_NAME);
    std::fs::create_dir(&public).unwrap();
    std::fs::set_permissions(
        &public,
        std::fs::Permissions::from_mode(CREATION_DIRECTORY_PUBLIC_MODE),
    )
    .unwrap();
    let error = verify_private_directory(
        &public.metadata().unwrap(),
        &public,
        CREATION_DIRECTORY_TEST_LABEL,
    )
    .unwrap_err();
    assert!(error.to_string().contains(CREATION_DIRECTORY_MODE_DETAIL));

    if unsafe { libc::geteuid() } != CREATION_DIRECTORY_ROOT_OWNER {
        let root = Path::new(CREATION_DIRECTORY_ROOT);
        let error = verify_private_directory(
            &root.metadata().unwrap(),
            root,
            CREATION_DIRECTORY_TEST_LABEL,
        )
        .unwrap_err();
        assert!(error.to_string().contains(CREATION_DIRECTORY_OWNER_DETAIL));
    }
}
