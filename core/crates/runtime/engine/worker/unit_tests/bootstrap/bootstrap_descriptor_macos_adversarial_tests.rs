use super::adversarial_support::{assert_released, reject_and_close_many, send_many};
use super::*;
use crate::constants::{
    TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT,
    TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
};
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::Duration;

const INVALID_CAPABILITY_MARKER: u8 = b'X';
const MULTIPLE_DESCRIPTOR_COUNT: usize = 2;
// XNU's 64-bit control mbuf accepts this many int descriptors after expansion.
const DARWIN_CONTROL_MBUF_DESCRIPTOR_LIMIT: usize = 254;
const KERNEL_LIMIT_HELPER_ENV: &str = "YOLO_KERNEL_LIMIT_DESCRIPTOR_HELPER";
const KERNEL_LIMIT_HELPER_TEST: &str = "worker::bootstrap::bootstrap_gate::descriptor::adversarial_tests::kernel_limit_descriptor_frame_is_rejected_and_closed";
const LOW_DESCRIPTOR_HELPER_ENV: &str = "YOLO_LOW_DESCRIPTOR_HELPER_FD";
const LOW_DESCRIPTOR_HELPER_TEST: &str =
    "worker::bootstrap::bootstrap_gate::descriptor::adversarial_tests::low_descriptor_helper";
const FCNTL_FAILURE: libc::c_int = -1;
const CLOSED_DESCRIPTOR_COUNT: libc::c_int = 3;
const RELEASE_PROBE_TIMEOUT: Duration = Duration::from_millis(25);
const RELEASE_COMPLETION_TIMEOUT: Duration = Duration::from_secs(2);

#[test]
fn malformed_marker_closes_the_received_writer() {
    let (manager, child) = std::os::unix::net::UnixStream::pair().unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    send_once(
        manager.as_raw_fd(),
        issuer.writer_descriptor(),
        INVALID_CAPABILITY_MARKER,
    )
    .unwrap();
    assert!(receive_once(child.as_raw_fd()).is_err());
    assert_released(issuer);
}

#[test]
fn release_observation_waits_for_an_active_spawn_boundary() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let spawn_guard = crate::process::waitable_child_spawn_guard().unwrap();
    let (attempting_sender, attempting_receiver) = channel();
    let (released_sender, released_receiver) = channel();
    let release = std::thread::spawn(move || {
        attempting_sender.send(()).unwrap();
        assert_released(issuer);
        released_sender.send(()).unwrap();
    });
    attempting_receiver.recv().unwrap();
    assert_eq!(
        released_receiver.recv_timeout(RELEASE_PROBE_TIMEOUT),
        Err(RecvTimeoutError::Timeout)
    );
    drop(spawn_guard);
    released_receiver
        .recv_timeout(RELEASE_COMPLETION_TIMEOUT)
        .unwrap();
    release.join().unwrap();
}

#[test]
fn multiple_descriptors_are_rejected_and_closed() {
    reject_and_close_many(MULTIPLE_DESCRIPTOR_COUNT);
}

#[test]
fn kernel_limit_descriptor_frame_is_rejected_and_closed() {
    if std::env::var_os(KERNEL_LIMIT_HELPER_ENV).is_some() {
        reject_and_close_many(DARWIN_CONTROL_MBUF_DESCRIPTOR_LIMIT);
        return;
    }
    let _spawn_guard = crate::process::waitable_child_spawn_guard().unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            KERNEL_LIMIT_HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
            TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(KERNEL_LIMIT_HELPER_ENV, KERNEL_LIMIT_HELPER_ENV)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn low_numbered_malformed_descriptor_is_closed() {
    let (manager, child) = std::os::unix::net::UnixStream::pair().unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let child_descriptor = child.as_raw_fd();
    let mut helper = spawn_low_descriptor_helper(child_descriptor);
    drop(child);
    send_once(
        manager.as_raw_fd(),
        issuer.writer_descriptor(),
        INVALID_CAPABILITY_MARKER,
    )
    .unwrap();
    assert!(helper.wait().unwrap().success());
    assert_released(issuer);
}

#[test]
fn low_numbered_multi_descriptor_rejection_closes_every_received_right() {
    let (manager, child) = std::os::unix::net::UnixStream::pair().unwrap();
    let issuers = (0..MULTIPLE_DESCRIPTOR_COUNT)
        .map(|_| MacosProcessCapabilityIssuer::create().unwrap())
        .collect::<Vec<_>>();
    let descriptors = issuers
        .iter()
        .map(MacosProcessCapabilityIssuer::writer_descriptor)
        .collect::<Vec<_>>();
    let mut helper = spawn_low_descriptor_helper(child.as_raw_fd());
    drop(child);
    send_many(manager.as_raw_fd(), &descriptors).unwrap();
    assert!(helper.wait().unwrap().success());
    for issuer in issuers {
        assert_released(issuer);
    }
}

#[test]
fn low_descriptor_helper() {
    let Some(raw) = std::env::var(LOW_DESCRIPTOR_HELPER_ENV)
        .ok()
        .and_then(|value| value.parse().ok())
    else {
        return;
    };
    let socket = unsafe { std::os::unix::net::UnixStream::from_raw_fd(raw) };
    for descriptor in libc::STDIN_FILENO..CLOSED_DESCRIPTOR_COUNT {
        unsafe { libc::close(descriptor) };
    }
    assert!(receive_once(socket.as_raw_fd()).is_err());
    for descriptor in libc::STDIN_FILENO..CLOSED_DESCRIPTOR_COUNT {
        assert_eq!(
            unsafe { libc::fcntl(descriptor, libc::F_GETFD) },
            FCNTL_FAILURE
        );
    }
}

fn low_descriptor_helper_command(child_descriptor: libc::c_int) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            LOW_DESCRIPTOR_HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
            TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(LOW_DESCRIPTOR_HELPER_ENV, child_descriptor.to_string());
    unsafe {
        command.pre_exec(move || clear_close_on_exec(child_descriptor));
    }
    command
}

fn spawn_low_descriptor_helper(child_descriptor: libc::c_int) -> Child {
    let _spawn_guard = crate::process::waitable_child_spawn_guard().unwrap();
    low_descriptor_helper_command(child_descriptor)
        .spawn()
        .unwrap()
}

fn clear_close_on_exec(descriptor: libc::c_int) -> std::io::Result<()> {
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    if flags == FCNTL_FAILURE {
        return Err(std::io::Error::last_os_error());
    }
    let changed = unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags & !libc::FD_CLOEXEC) };
    if changed == FCNTL_FAILURE {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
