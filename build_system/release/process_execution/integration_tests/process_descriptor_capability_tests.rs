use super::process_supervision_tests::Fixture;
use crate::constants::{DESCRIPTOR_PATH_PREFIX, FCNTL_ERROR};
use crate::posix_spawn_descriptors::IsolatedSpawnDescriptors;
use std::os::fd::AsRawFd;
use yolo_process::ProcessDiscoveryCapability;

const TEST_EXECUTABLE: &str = "/usr/bin/true";
const FILE_TYPE_TEST_EXECUTABLE: &str = "/bin/test";
const NAMED_PIPE_TEST_ARGUMENT: &str = "-p";
const TEST_DESCRIPTION: &str = "process descriptor capability mapping";

#[test]
fn capability_gets_a_distinct_inherited_child_descriptor() {
    let fixture = Fixture::new();
    let command = fixture.command(TEST_EXECUTABLE, Vec::new(), TEST_DESCRIPTION);
    let standard_output = tempfile::tempfile().unwrap();
    let standard_error = tempfile::tempfile().unwrap();
    let capability = ProcessDiscoveryCapability::generate().unwrap();
    let inheritance = capability.begin_writer_inheritance().unwrap();
    let source = inheritance.descriptor();
    let isolated = IsolatedSpawnDescriptors::new(
        &command,
        &standard_output,
        &standard_error,
        None,
        Some(source),
    )
    .unwrap();
    let [binding] = isolated.inherited.as_slice() else {
        panic!("exactly one process capability descriptor must be mapped");
    };
    assert_ne!(binding.source.as_raw_fd(), binding.child_descriptor);
    assert!(binding.child_descriptor > libc::STDERR_FILENO);
    let flags = unsafe { libc::fcntl(binding.source.as_raw_fd(), libc::F_GETFD) };
    assert_ne!(flags, FCNTL_ERROR);
    assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
}

#[test]
fn capability_survives_cloexec_default_in_the_spawned_child() {
    let fixture = Fixture::new();
    let descriptor = libc::STDERR_FILENO + 1;
    let descriptor_path = format!("{DESCRIPTOR_PATH_PREFIX}/{descriptor}");
    let command = fixture.command(
        FILE_TYPE_TEST_EXECUTABLE,
        vec![NAMED_PIPE_TEST_ARGUMENT.to_owned(), descriptor_path],
        TEST_DESCRIPTION,
    );
    let output = command.execute().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
