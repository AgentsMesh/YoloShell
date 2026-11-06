use super::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};

const SOURCE_FILE_NAME: &str = "retained-clone-source";
const TARGET_FILE_NAME: &str = "retained-clone-target";
const CLONE_PAYLOAD: &[u8] = b"retained clone payload";

#[test]
fn anonymous_clone_is_validated_before_and_after_linking() {
    let root = tempfile::tempdir().unwrap();
    let directory = PrivateDirectory::ensure(root.path()).unwrap();
    let source_path = root.path().join(SOURCE_FILE_NAME);
    let target_path = root.path().join(TARGET_FILE_NAME);
    let mut source = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(&source_path)
        .unwrap();
    source.write_all(CLONE_PAYLOAD).unwrap();
    source.sync_all().unwrap();
    let target_name =
        crate::paths::directory::entry_c_string(target_path.file_name().unwrap()).unwrap();

    clone_platform_file(
        &source,
        directory.descriptor(),
        &target_name,
        &target_path,
        u64::try_from(CLONE_PAYLOAD.len()).unwrap(),
    )
    .unwrap();

    assert_eq!(fs::read(&target_path).unwrap(), CLONE_PAYLOAD);
    assert_eq!(
        fs::metadata(target_path).unwrap().nlink(),
        crate::paths::constants::EXPECTED_FILE_LINK_COUNT
    );
}
