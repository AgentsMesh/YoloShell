use super::*;

const DEVICE: u64 = 7;
const INODE: u64 = 11;
const FIRST_BIRTH_SECOND: i64 = 13;
const SECOND_BIRTH_SECOND: i64 = 17;
const BIRTH_NANOSECOND: i64 = 19;

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn reused_device_and_inode_require_the_same_birth_generation() {
    let first =
        ObjectIdentity::from_components(DEVICE, INODE, FIRST_BIRTH_SECOND, BIRTH_NANOSECOND);
    let reused =
        ObjectIdentity::from_components(DEVICE, INODE, SECOND_BIRTH_SECOND, BIRTH_NANOSECOND);
    assert_ne!(first, reused);
}

#[cfg(target_os = "linux")]
#[test]
fn linux_files_require_kernel_birth_identity() {
    let file = tempfile::tempfile().unwrap();
    assert!(from_file(&file).unwrap().is_some());
}
