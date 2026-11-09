use super::*;
use crate::unit_support::*;
use std::os::unix::fs::MetadataExt;
use std::sync::Arc;

const DESCRIPTOR_STEP: libc::c_int = 1;

#[test]
fn duplicate_retains_identity_and_rejects_invalid_sources() {
    let file = tempfile::tempfile().unwrap();
    let cloned = duplicate(file.as_raw_fd(), file.as_raw_fd()).unwrap();
    assert!(cloned.as_raw_fd() > file.as_raw_fd());
    assert_eq!(
        cloned.metadata().unwrap().ino(),
        file.metadata().unwrap().ino()
    );
    assert_ne!(
        unsafe { libc::fcntl(cloned.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
        0
    );
    let error = duplicate(INVALID_DESCRIPTOR, libc::STDERR_FILENO)
        .err()
        .unwrap();
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(libc::EBADF)
    );
}

#[test]
fn descriptor_range_checks_both_capability_and_source_boundaries() {
    let fixture = Fixture::new();
    let file = Arc::new(tempfile::tempfile().unwrap());
    let command = fixture.command();
    assert_eq!(
        safe_descriptor_minimum(&command).unwrap(),
        libc::STDERR_FILENO + DESCRIPTOR_STEP
    );
    for destination in [libc::c_int::MAX, libc::c_int::MAX - DESCRIPTOR_STEP] {
        let command = command.clone().inherit_as(Arc::clone(&file), destination);
        let error = IsolatedSpawnDescriptors::new(&command, &file, &file, None, None)
            .err()
            .unwrap();
        assert!(error.to_string().contains(DESCRIPTOR_RANGE_ERROR));
    }
}

#[test]
fn optional_input_and_capability_use_separate_owned_descriptors() {
    let fixture = Fixture::new();
    let file = tempfile::tempfile().unwrap();
    let command = fixture.command();
    let isolated = IsolatedSpawnDescriptors::new(&command, &file, &file, None, None).unwrap();
    assert!(isolated.standard_input.is_none());
    assert!(isolated.inherited.is_empty());
    let isolated =
        IsolatedSpawnDescriptors::new(&command, &file, &file, Some(&file), Some(file.as_raw_fd()))
            .unwrap();
    let input = isolated.standard_input.unwrap();
    assert_ne!(input.as_raw_fd(), file.as_raw_fd());
    let capability = isolated.inherited.first().unwrap();
    assert_eq!(
        capability.child_descriptor,
        safe_descriptor_minimum(&command).unwrap()
    );
    assert!(capability.source.as_raw_fd() > capability.child_descriptor);
    assert!(
        IsolatedSpawnDescriptors::new(&command, &file, &file, None, Some(INVALID_DESCRIPTOR))
            .is_err()
    );
}
