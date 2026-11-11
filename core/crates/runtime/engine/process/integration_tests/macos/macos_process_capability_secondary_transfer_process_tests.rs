use super::*;
use crate::process::{LiveProcessIdentity, ProcessIdentity};
use crate::runtime_constants::{
    TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT,
    TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
};
use std::os::unix::net::UnixStream;
use std::time::Duration;

const HELPER_TEST: &str = "macos_process_capability::\
secondary_transfer_process_tests::descriptor_receiver_helper";
const HELPER_ROLE_ENV: &str = "YOLO_CAPABILITY_RECEIVER_ROLE";
const HELPER_SOCKET_ENV: &str = "YOLO_CAPABILITY_RECEIVER_SOCKET";
const HELPER_READER_ENV: &str = "YOLO_CAPABILITY_RECEIVER_READER";
const HELPER_WRITER_ENV: &str = "YOLO_CAPABILITY_RECEIVER_WRITER";
const HELPER_READY_ENV: &str = "YOLO_CAPABILITY_RECEIVER_READY";
const HELPER_ROLE: &str = "secondary-custody-host";
const SHELL_COMMAND: &str = "/bin/sh";
const SHELL_SCRIPT: &str = "trap '' HUP TERM; while :; do sleep 1; done";
const RIGHTS_MARKER: u8 = b'C';
const READY_COMPONENT: &str = "ready";
const SOCKET_COMPONENT: &str = "custody.sock";
const EMPTY_MARKER: &[u8] = b"";
const HELPER_TIMEOUT: Duration = Duration::from_secs(5);
const HELPER_TIMEOUT_DETAIL: &str = "secondary custody descriptor helper timed out";

#[path = "../../test_support/secondary_transfer_process.rs"]
mod fixture;
use fixture::{Receiver, receive_descriptor, required_path, required_u64, spawn_shell_holder};

#[test]
fn reader_survives_first_host_sigkill_and_exact_second_transfer() {
    let (reader, writer) = MacosProcessCapabilityIssuer::create()
        .unwrap()
        .into_staged()
        .unwrap();
    let mut shell = spawn_shell_holder(&writer);
    let root = LiveProcessIdentity::capture(ProcessIdentity::capture(shell.id()).unwrap())
        .unwrap()
        .unwrap();
    reader.verify_writer_holder(&root).unwrap();
    drop(writer);

    let mut first = Receiver::spawn(reader.as_descriptor(), reader.identity());
    first.wait_ready();
    first.kill_and_wait();
    reader.verify_writer_holder(&root).unwrap();

    let mut second = Receiver::spawn(reader.as_descriptor(), reader.identity());
    second.wait_ready();
    reader.verify_writer_holder(&root).unwrap();
    second.kill_and_wait();

    shell.kill().unwrap();
    shell.wait().unwrap();
}

#[test]
fn released_writer_preserves_exact_reader_across_host_replacement() {
    let (reader, writer) = MacosProcessCapabilityIssuer::create()
        .unwrap()
        .into_staged()
        .unwrap();
    let mut first = Receiver::spawn(reader.as_descriptor(), reader.identity());
    first.wait_ready();

    drop(writer);
    first.kill_and_wait();

    let mut second = Receiver::spawn(reader.as_descriptor(), reader.identity());
    second.wait_ready();
    second.kill_and_wait();
}

#[test]
fn descriptor_receiver_helper() {
    if std::env::var(HELPER_ROLE_ENV).as_deref() != Ok(HELPER_ROLE) {
        return;
    }
    let stream = UnixStream::connect(required_path(HELPER_SOCKET_ENV)).unwrap();
    let descriptor = receive_descriptor(&stream);
    let expected = MacosProcessCapabilityIdentity {
        reader_handle: required_u64(HELPER_READER_ENV),
        writer_handle: required_u64(HELPER_WRITER_ENV),
    };
    let _reader =
        MacosProcessCapabilityManagerReader::from_transferred_attested(descriptor, expected)
            .unwrap();
    std::fs::write(required_path(HELPER_READY_ENV), EMPTY_MARKER).unwrap();
    loop {
        std::thread::park();
    }
}
