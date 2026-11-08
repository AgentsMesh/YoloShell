use super::process_supervision_tests::Fixture;
use super::support::error_text;
use super::test_constants::{BASH_EXECUTABLE, SHELL_COMMAND_ARGUMENT};
use crate::constants::{DESCRIPTOR_PATH_PREFIX, FCNTL_ERROR};
use crate::posix_spawn_descriptors::{DESCRIPTOR_RANGE_ERROR, IsolatedSpawnDescriptors};
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::FileExt;
use std::sync::Arc;

const DESCRIPTOR_FLOOR: libc::c_int = 700;
const CAT_EXECUTABLE: &str = "/bin/cat";
const TEST_EXECUTABLE: &str = "/bin/test";
const MAPPING_DESCRIPTION: &str = "collision-free descriptor mapping";
const FIRST_BYTES: &[u8] = b"first\n";
const SECOND_BYTES: &[u8] = b"second\n";
const THIRD_BYTES: &[u8] = b"third\n";

#[test]
fn crossed_source_and_destination_descriptors_preserve_both_files() {
    let first = source(FIRST_BYTES);
    let second = source(SECOND_BYTES);
    let output = run(
        &[
            (Arc::clone(&second), first.as_raw_fd()),
            (Arc::clone(&first), second.as_raw_fd()),
        ],
        &[first.as_raw_fd(), second.as_raw_fd()],
    );
    assert_eq!(output, [SECOND_BYTES, FIRST_BYTES].concat());
}

#[test]
fn cyclic_source_and_destination_descriptors_preserve_every_file() {
    let first = source(FIRST_BYTES);
    let second = source(SECOND_BYTES);
    let third = source(THIRD_BYTES);
    let output = run(
        &[
            (Arc::clone(&second), first.as_raw_fd()),
            (Arc::clone(&third), second.as_raw_fd()),
            (Arc::clone(&first), third.as_raw_fd()),
        ],
        &[first.as_raw_fd(), second.as_raw_fd(), third.as_raw_fd()],
    );
    assert_eq!(output, [SECOND_BYTES, THIRD_BYTES, FIRST_BYTES].concat());
}

#[test]
fn duplicate_source_mappings_retain_the_original_open_file_description() {
    let shared = source(FIRST_BYTES);
    let other = source(SECOND_BYTES);
    let reservation = source(THIRD_BYTES);
    let destinations = [other.as_raw_fd(), reservation.as_raw_fd()];
    let script = format!(
        "{TEST_EXECUTABLE} {DESCRIPTOR_PATH_PREFIX}/{} -ef {DESCRIPTOR_PATH_PREFIX}/{}\n\
         {CAT_EXECUTABLE} {DESCRIPTOR_PATH_PREFIX}/{}",
        destinations[0], destinations[1], destinations[0]
    );
    let output = execute(
        &[
            (Arc::clone(&other), shared.as_raw_fd()),
            (Arc::clone(&shared), destinations[0]),
            (Arc::clone(&shared), destinations[1]),
        ],
        script,
    );
    assert_eq!(output, FIRST_BYTES);
}

#[test]
fn standard_and_inherited_sources_are_cloexec_isolated_and_dropped() {
    let fixture = Fixture::new();
    let standard_output = tempfile::tempfile().unwrap();
    let standard_error = tempfile::tempfile().unwrap();
    let standard_input = tempfile::tempfile().unwrap();
    let shared = source(FIRST_BYTES);
    let command = fixture
        .command(CAT_EXECUTABLE, Vec::new(), MAPPING_DESCRIPTION)
        .inherit_as(Arc::clone(&shared), shared.as_raw_fd())
        .inherit_as(shared, DESCRIPTOR_FLOOR);
    let isolated = IsolatedSpawnDescriptors::new(
        &command,
        &standard_output,
        &standard_error,
        Some(&standard_input),
        None,
    )
    .unwrap();
    let maximum = command
        .inherited
        .iter()
        .map(|binding| binding.child_descriptor)
        .max()
        .unwrap();
    let descriptors = isolated_descriptors(&isolated);
    assert!(descriptors.iter().all(|descriptor| *descriptor > maximum));
    assert!(
        descriptors
            .iter()
            .all(|descriptor| close_on_exec(*descriptor))
    );
    drop(isolated);
    assert!(descriptors.iter().all(|descriptor| closed(*descriptor)));
}

#[test]
fn maximum_child_descriptor_is_rejected_without_wrapping() {
    let fixture = Fixture::new();
    let standard_output = tempfile::tempfile().unwrap();
    let standard_error = tempfile::tempfile().unwrap();
    let source = source(FIRST_BYTES);
    let command = fixture
        .command(CAT_EXECUTABLE, Vec::new(), MAPPING_DESCRIPTION)
        .inherit_as(source, libc::c_int::MAX);
    let error = error_text(IsolatedSpawnDescriptors::new(
        &command,
        &standard_output,
        &standard_error,
        None,
        None,
    ));
    assert!(error.contains(DESCRIPTOR_RANGE_ERROR), "{error}");
}

fn run(bindings: &[(Arc<File>, libc::c_int)], descriptors: &[libc::c_int]) -> Vec<u8> {
    let script = descriptors
        .iter()
        .map(|descriptor| format!("{CAT_EXECUTABLE} {DESCRIPTOR_PATH_PREFIX}/{descriptor}"))
        .collect::<Vec<_>>()
        .join("\n");
    execute(bindings, script)
}

fn execute(bindings: &[(Arc<File>, libc::c_int)], script: String) -> Vec<u8> {
    let fixture = Fixture::new();
    let mut command = fixture.command(
        BASH_EXECUTABLE,
        vec![SHELL_COMMAND_ARGUMENT.to_owned(), script],
        MAPPING_DESCRIPTION,
    );
    for (source, destination) in bindings {
        command = command.inherit_as(Arc::clone(source), *destination);
    }
    let output = command.execute().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn source(bytes: &[u8]) -> Arc<File> {
    let original = tempfile::tempfile().unwrap();
    original.write_all_at(bytes, 0).unwrap();
    let descriptor = unsafe {
        libc::fcntl(
            original.as_raw_fd(),
            libc::F_DUPFD_CLOEXEC,
            DESCRIPTOR_FLOOR,
        )
    };
    assert_ne!(descriptor, FCNTL_ERROR);
    Arc::new(unsafe { File::from_raw_fd(descriptor) })
}

fn isolated_descriptors(isolated: &IsolatedSpawnDescriptors) -> Vec<libc::c_int> {
    [
        isolated.standard_output.as_raw_fd(),
        isolated.standard_error.as_raw_fd(),
        isolated.standard_input.as_ref().unwrap().as_raw_fd(),
    ]
    .into_iter()
    .chain(
        isolated
            .inherited
            .iter()
            .map(|binding| binding.source.as_raw_fd()),
    )
    .collect()
}

fn close_on_exec(descriptor: libc::c_int) -> bool {
    (unsafe { libc::fcntl(descriptor, libc::F_GETFD) }) & libc::FD_CLOEXEC != 0
}

fn closed(descriptor: libc::c_int) -> bool {
    (unsafe { libc::fcntl(descriptor, libc::F_GETFD) }) == FCNTL_ERROR
}
