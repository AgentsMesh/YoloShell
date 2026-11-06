use super::child_lifecycle::{SLEEP_ARGUMENT, SLEEP_EXECUTABLE, assert_reaped};
use super::process_supervision_tests::Fixture;
use super::test_constants::PROCESS_CLEANUP_OBSERVATION_LIMIT_SECONDS;
use crate::constants::PROCESS_DRAIN_GRACE;
use crate::posix_cleanup::terminate_process;
use crate::posix_spawn::spawn;
use crate::posix_supervisor::Streams;
use crate::process_descriptor::{set_close_on_exec, set_nonblocking};
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd};
use std::time::{Duration, Instant};

const CHILD_DESCRIPTION: &str = "child with externally retained output writer";
const PIPE_ENDPOINT_COUNT: usize = 2;
const PIPE_READER_INDEX: usize = 0;
const PIPE_WRITER_INDEX: usize = 1;
const SYSCALL_SUCCESS: libc::c_int = 0;
const OUTPUT_MARKER: &[u8] = b"output before drain deadline";
const EOF_FAILURE: &str = "spawned process pipes did not reach EOF";

#[test]
fn retained_pipe_writer_reaches_drain_deadline_without_hiding_reaped_child() {
    let fixture = Fixture::new();
    let command = fixture.command(
        SLEEP_EXECUTABLE,
        vec![SLEEP_ARGUMENT.into()],
        CHILD_DESCRIPTION,
    );
    let mut child = spawn(&command).unwrap();
    let pid = child.pid;
    let (reader, mut retained_writer) = output_pipe();
    retained_writer.write_all(OUTPUT_MARKER).unwrap();
    child.stdout = reader;
    let mut streams = Streams::default();

    let started = Instant::now();
    let error = terminate_process(&mut child, &mut streams).unwrap_err();
    let elapsed = started.elapsed();

    assert_eq!(error.to_string(), EOF_FAILURE);
    assert!(elapsed >= PROCESS_DRAIN_GRACE);
    assert!(elapsed < Duration::from_secs(PROCESS_CLEANUP_OBSERVATION_LIMIT_SECONDS));
    assert_reaped(pid);
    assert_eq!(streams.stdout.bytes, OUTPUT_MARKER);
    assert!(!streams.complete());

    drop(retained_writer);
    streams.drain(&mut child).unwrap();
    assert!(streams.complete());
    assert_eq!(streams.stdout.bytes, OUTPUT_MARKER);
}

#[test]
fn cleanup_error_with_unconfirmed_output_retains_all_command_resources() {
    let owners = super::resources::Resources::new();
    let paths = owners.paths();
    let command = owners.command(SLEEP_EXECUTABLE, &[SLEEP_ARGUMENT]);
    let mut child = spawn(&command).unwrap();
    let pid = child.pid;
    let (reader, retained_writer) = output_pipe();
    child.stdout = reader;
    let error = terminate_process(&mut child, &mut Streams::default()).unwrap_err();
    assert_eq!(error.to_string(), EOF_FAILURE);
    assert_reaped(pid);
    drop(child);
    drop(owners);
    let preserved = paths.iter().all(|path| path.is_dir());
    drop(retained_writer);
    assert!(
        preserved,
        "unconfirmed subprocess settlement deleted its resources"
    );
    for path in paths {
        std::fs::remove_dir_all(path).unwrap();
    }
}

fn output_pipe() -> (File, File) {
    let mut descriptors = [libc::c_int::default(); PIPE_ENDPOINT_COUNT];
    assert_eq!(
        unsafe { libc::pipe(descriptors.as_mut_ptr()) },
        SYSCALL_SUCCESS
    );
    let reader = unsafe { File::from_raw_fd(descriptors[PIPE_READER_INDEX]) };
    let writer = unsafe { File::from_raw_fd(descriptors[PIPE_WRITER_INDEX]) };
    set_close_on_exec(reader.as_raw_fd()).unwrap();
    set_close_on_exec(writer.as_raw_fd()).unwrap();
    set_nonblocking(reader.as_raw_fd()).unwrap();
    (reader, writer)
}
