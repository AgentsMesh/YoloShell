use super::*;
use std::os::unix::fs::MetadataExt;

#[test]
fn current_process_metadata_and_mapped_executable_are_consistent() {
    let pid = std::process::id();
    let before = process_metadata(pid).unwrap();
    assert_eq!(before.pid, pid);
    assert_eq!(before.parent_pid, unsafe { libc::getppid() } as u32);
    assert_eq!(before.start_time, crate::process_start_time(pid).unwrap());
    let mapped = open_executable(pid).unwrap().metadata().unwrap();
    let path = executable_path(pid).unwrap();
    let named = std::fs::metadata(path).unwrap();
    assert_eq!((mapped.dev(), mapped.ino()), (named.dev(), named.ino()));
    assert_eq!(process_metadata(pid).unwrap(), before);
}

#[test]
fn invalid_and_missing_processes_have_no_native_executable_authority() {
    for pid in [u32::default(), u32::MAX] {
        assert!(process_metadata(pid).is_err());
        assert!(executable_path(pid).is_err());
        assert!(open_executable(pid).is_err());
    }
}
