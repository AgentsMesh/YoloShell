use super::*;
use crate::process::MacosProcessCapabilityIssuer;
use crate::runtime_constants::{
    TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT,
    TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
};
use std::time::{Duration, Instant};

const SHELL_ENV: &str = "YOLO_CAPABILITY_EXEC_SHELL";
const WRITER_ENV: &str = "YOLO_CAPABILITY_EXEC_WRITER";
const EXPECTED_WRITER_ENV: &str = "YOLO_CAPABILITY_EXEC_EXPECTED_WRITER";
const HOME_ENV: &str = "HOME";
const SHELL_EXEC_SCRIPT: &str = "exec \"$@\"";
const SHELL_ARGV_ZERO: &str = "yolo-capability-shell";
const DIRECT_EXEC: &str = "direct-exec";
#[path = "../../test_support/shell_exec_constants.rs"]
mod constants;
use constants::INSPECT_PHASE;
const SHELLS: &[&str] = &[DIRECT_EXEC, "/bin/sh", "/bin/zsh"];
const TEST_NAME: &str = "macos_process_capability::exec::shell_tests::successful_shell_exec_keeps_writer_descriptor_open";
const CHILD_TIMEOUT: Duration = Duration::from_secs(10);
const CHILD_POLL: Duration = Duration::from_millis(10);

#[test]
fn transferred_high_writer_keeps_descriptor_number_and_identity() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let descriptor = unsafe {
        libc::fcntl(
            issuer.writer_descriptor(),
            libc::F_DUPFD_CLOEXEC,
            crate::process::macos_process_capability_fd::MINIMUM_INHERITED_WRITER_DESCRIPTOR,
        )
    };
    assert_ne!(descriptor, FCNTL_FAILURE);
    let writer = MacosProcessCapabilityChildWriter::from_transferred(unsafe {
        OwnedFd::from_raw_fd(descriptor)
    })
    .unwrap();
    assert_eq!(writer.descriptor.as_raw_fd(), descriptor);
    assert_eq!(writer.identity(), issuer.identity());
    assert_ne!(
        unsafe { libc::fcntl(descriptor, libc::F_GETFD) } & libc::FD_CLOEXEC,
        0
    );
}

#[test]
fn successful_shell_exec_keeps_writer_descriptor_open() {
    if std::env::var(SHELL_ENV).as_deref() == Ok(INSPECT_PHASE) {
        let expected = std::env::var(EXPECTED_WRITER_ENV)
            .unwrap()
            .parse::<u64>()
            .unwrap();
        assert!(crate::process::pty_session_proof::macos_descriptor_capability::
            exact_descriptor_holder(std::process::id(), expected).unwrap(),
            "shell exec lost the original pipe writer identity");
        return;
    }
    if let Some(shell) = std::env::var_os(SHELL_ENV) {
        let descriptor: libc::c_int = std::env::var(WRITER_ENV).unwrap().parse().unwrap();
        assert_ne!(
            unsafe { libc::fcntl(descriptor, libc::F_GETFD) },
            FCNTL_FAILURE
        );
        let mut writer = MacosProcessCapabilityChildWriter::from_transferred(unsafe {
            OwnedFd::from_raw_fd(descriptor)
        })
        .unwrap();
        let executable = std::env::current_exe().unwrap();
        let mut command = if shell == DIRECT_EXEC {
            Command::new(&executable)
        } else {
            let mut command = Command::new(shell);
            command
                .args(["-l", "-i", "-c", SHELL_EXEC_SCRIPT, SHELL_ARGV_ZERO])
                .arg(&executable);
            command
        };
        command
            .args([
                TEST_HARNESS_EXACT_ARGUMENT,
                TEST_NAME,
                TEST_HARNESS_NOCAPTURE_ARGUMENT,
                TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
            ])
            .env(SHELL_ENV, INSPECT_PHASE);
        panic!("{}", writer.exec_inheriting(&mut command));
    }
    for shell in SHELLS {
        let issuer = MacosProcessCapabilityIssuer::create().unwrap();
        let (_reader, writer) = super::super::test_support::transfer(issuer).unwrap();
        let home = tempfile::tempdir().unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                TEST_HARNESS_EXACT_ARGUMENT,
                TEST_NAME,
                TEST_HARNESS_NOCAPTURE_ARGUMENT,
                TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
            ])
            .env(SHELL_ENV, shell)
            .env(WRITER_ENV, writer.descriptor.as_raw_fd().to_string())
            .env(
                EXPECTED_WRITER_ENV,
                writer.identity().writer_handle().to_string(),
            )
            .env(HOME_ENV, home.path());
        writer.configure_exec_inheritance(&mut command);
        let mut child = crate::process::waitable_child_spawn_guard()
            .unwrap()
            .boundary()
            .spawn_command(&mut command)
            .unwrap();
        let deadline = Instant::now() + CHILD_TIMEOUT;
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "capability writer was lost by {shell}: {status}"
                );
                break;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("shell exec probe timed out: {shell}");
            }
            std::thread::sleep(CHILD_POLL);
        }
    }
}
