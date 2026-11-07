use super::*;
use std::os::fd::AsRawFd;
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::Duration;

const FCNTL_FAILURE: libc::c_int = -1;
const WRITTEN_BYTE_COUNT: libc::ssize_t = 1;
const SPAWN_GATE_PROBE_TIMEOUT: Duration = Duration::from_millis(25);
const SPAWN_GATE_RELEASE_TIMEOUT: Duration = Duration::from_secs(2);

#[test]
fn issuer_separates_reader_and_writer_roles() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let reader = issuer.reader.as_ref().unwrap().as_raw_fd();
    let writer = issuer.writer.as_ref().unwrap().as_raw_fd();
    assert_endpoint(reader, libc::O_RDONLY, true);
    assert_endpoint(writer, libc::O_WRONLY, true);
    assert_ne!(
        issuer.identity().reader_handle(),
        issuer.identity().writer_handle()
    );
}

#[test]
fn committed_reader_proves_last_writer_release() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = test_support::transfer(issuer).unwrap();
    assert_eq!(
        reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::WriterPresent
    );
    drop(writer);
    assert_eq!(
        reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::Released
    );
}

#[test]
fn acknowledgement_must_name_both_exact_endpoints() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let identity = issuer.identity();
    assert!(
        issuer
            .commit_acknowledgement(identity.reader_handle(), u64::default())
            .is_err()
    );
}

#[test]
fn unexpected_writer_data_never_spoofs_release() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = test_support::transfer(issuer).unwrap();
    let byte = b'!';
    let written = unsafe {
        libc::write(
            writer.descriptor.as_raw_fd(),
            std::ptr::addr_of!(byte).cast(),
            std::mem::size_of_val(&byte),
        )
    };
    assert_eq!(written, WRITTEN_BYTE_COUNT);
    assert!(reader.poll_release().is_err());
}

#[test]
fn issuer_creation_is_serialized_against_the_repo_spawn_gate() {
    let gate = crate::process::waitable_child_spawn_guard().unwrap();
    let (started_sender, started_receiver) = channel();
    let (result_sender, result_receiver) = channel();
    let thread = std::thread::spawn(move || {
        started_sender.send(()).unwrap();
        result_sender
            .send(MacosProcessCapabilityIssuer::create())
            .unwrap();
    });
    started_receiver
        .recv_timeout(SPAWN_GATE_RELEASE_TIMEOUT)
        .unwrap();
    assert!(matches!(
        result_receiver.recv_timeout(SPAWN_GATE_PROBE_TIMEOUT),
        Err(RecvTimeoutError::Timeout)
    ));
    drop(gate);
    assert!(
        result_receiver
            .recv_timeout(SPAWN_GATE_RELEASE_TIMEOUT)
            .unwrap()
            .is_ok()
    );
    thread.join().unwrap();
}

fn assert_endpoint(descriptor: libc::c_int, direction: libc::c_int, close_on_exec: bool) {
    let descriptor_flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    let status_flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    assert_ne!(descriptor_flags, FCNTL_FAILURE);
    assert_ne!(status_flags, FCNTL_FAILURE);
    assert_eq!(
        descriptor_flags & libc::FD_CLOEXEC != libc::c_int::default(),
        close_on_exec
    );
    assert_eq!(status_flags & libc::O_ACCMODE, direction);
    assert_ne!(status_flags & libc::O_NONBLOCK, libc::c_int::default());
}
