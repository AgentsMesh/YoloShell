use super::test_support::{assert_reader_eof, install, pipe, send_right};
use super::{BlockingLocalUnixStream, LocalUnixStream, UnexpectedAncillaryData};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, OwnedFd};
use std::process::{Command, Stdio};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const TEST_MARKER: u8 = b'T';
const TRUNCATION_TEST_ROLE: &str = "YOLO_LOCAL_UNIX_TRUNCATION_TEST_ROLE";
const TRUNCATION_TEST_ROLE_VALUE: &str = "enabled";
const TRUNCATION_TEST_NAME: &str = "stream_tests::truncated_control_fail_stops_the_process";

#[tokio::test]
async fn async_stream_debug_is_opaque() {
    let (stream, _) = LocalUnixStream::pair().unwrap();
    assert_eq!(
        format!("{stream:?}"),
        concat!(stringify!(LocalUnixStream), " { .. }")
    );
}

#[test]
fn truncated_control_fail_stops_the_process() {
    if std::env::var_os(TRUNCATION_TEST_ROLE).is_some() {
        super::ancillary_fail_stop::terminate_if_truncated(libc::MSG_CTRUNC);
        unreachable!();
    }
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            yolo_application_support::constants::TEST_HARNESS_EXACT_ARGUMENT,
            TRUNCATION_TEST_NAME,
        ])
        .env(TRUNCATION_TEST_ROLE, TRUNCATION_TEST_ROLE_VALUE)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = yolo_process_spawn::waitable_child_spawn_guard()
        .unwrap()
        .boundary()
        .spawn_command(&mut command)
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(libc::EXIT_FAILURE));
    assert_eq!(output.stderr, super::ancillary_fail_stop::DIAGNOSTIC);
}

#[tokio::test]
async fn async_read_rejects_and_closes_an_unexpected_right() {
    let (sender, receiver) = tokio::net::UnixStream::pair().unwrap();
    let pipe = pipe();
    send_right(sender.as_raw_fd(), pipe.writer.as_raw_fd(), TEST_MARKER);
    drop(pipe.writer);
    let mut receiver = LocalUnixStream::from(receiver);
    let mut byte = u8::default();
    let error = receiver
        .read(std::slice::from_mut(&mut byte))
        .await
        .unwrap_err();
    assert!(is_unexpected(&error));
    assert_reader_eof(&pipe.reader);
    assert!(
        receiver
            .read(std::slice::from_mut(&mut byte))
            .await
            .is_err()
    );
    assert!(receiver.write_all(&[TEST_MARKER]).await.is_err());
}

#[tokio::test]
async fn readiness_read_rejects_and_closes_an_unexpected_right() {
    let (sender, receiver) = tokio::net::UnixStream::pair().unwrap();
    let pipe = pipe();
    send_right(sender.as_raw_fd(), pipe.writer.as_raw_fd(), TEST_MARKER);
    drop(pipe.writer);
    let mut receiver = LocalUnixStream::from(receiver);
    receiver.readable().await.unwrap();
    let mut byte = u8::default();
    let error = receiver
        .try_read(std::slice::from_mut(&mut byte))
        .unwrap_err();
    assert!(is_unexpected(&error));
    assert_reader_eof(&pipe.reader);
    assert!(receiver.readable().await.is_err());
}

#[test]
fn blocking_read_rejects_and_closes_an_unexpected_right() {
    let (sender, receiver) = std::os::unix::net::UnixStream::pair().unwrap();
    let pipe = pipe();
    send_right(sender.as_raw_fd(), pipe.writer.as_raw_fd(), TEST_MARKER);
    drop(pipe.writer);
    let mut receiver = BlockingLocalUnixStream::from(receiver);
    let mut clone = receiver.try_clone().unwrap();
    let mut byte = u8::default();
    let error = receiver.read(std::slice::from_mut(&mut byte)).unwrap_err();
    assert!(is_unexpected(&error));
    assert_reader_eof(&pipe.reader);
    assert!(clone.read(std::slice::from_mut(&mut byte)).is_err());
    assert!(clone.write_all(&[TEST_MARKER]).is_err());
}

#[test]
fn rejected_descriptor_security_holds_spawn_gate_until_closure() {
    let (sender, receiver) = std::os::unix::net::UnixStream::pair().unwrap();
    let pipe = pipe();
    send_right(sender.as_raw_fd(), pipe.writer.as_raw_fd(), TEST_MARKER);
    drop(pipe.writer);
    let pause = install(receiver.as_raw_fd());
    let reader = std::thread::spawn(move || {
        let mut receiver = BlockingLocalUnixStream::from(receiver);
        let mut byte = u8::default();
        receiver.read(std::slice::from_mut(&mut byte)).unwrap_err()
    });

    pause.wait_until_reached();
    let spawn_gate_was_held = super::test_support::spawn_gate_is_held();
    pause.release();
    let error = reader.join().unwrap();

    assert!(spawn_gate_was_held);
    assert!(is_unexpected(&error));
    assert_reader_eof(&pipe.reader);
}

#[test]
fn expected_right_is_owned_and_close_on_exec() {
    let (sender, receiver) = std::os::unix::net::UnixStream::pair().unwrap();
    let pipe = pipe();
    send_right(sender.as_raw_fd(), pipe.writer.as_raw_fd(), TEST_MARKER);
    let received =
        super::recv_macos::receive_exactly_one_right(receiver.as_raw_fd(), TEST_MARKER).unwrap();
    assert_close_on_exec(&received);
    drop(pipe.writer);
    drop(received);
    assert_reader_eof(&pipe.reader);
}

#[test]
fn fixture_pipe_cannot_escape_into_parallel_exec() {
    let pipe = pipe();
    assert_close_on_exec(&pipe.reader);
    assert_close_on_exec(&pipe.writer);
}

fn is_unexpected(error: &std::io::Error) -> bool {
    error
        .get_ref()
        .is_some_and(|source| source.is::<UnexpectedAncillaryData>())
}

fn assert_close_on_exec(descriptor: &OwnedFd) {
    let flags = unsafe { libc::fcntl(descriptor.as_raw_fd(), libc::F_GETFD) };
    assert_ne!(flags, super::constants::FCNTL_FAILURE);
    assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
}
