use super::*;
use crate::process::MacosProcessCapabilityIssuer;
use crate::runtime_constants::{
    TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT,
    TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
};
use std::mem::MaybeUninit;
use std::os::fd::AsRawFd;
use std::process::Child;
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::Duration;

const MISSING_EXECUTABLE: &str = "/definitely/missing/yolo-capability-exec";
const ISOLATED_EXEC_TEST_ENVIRONMENT: &str = "YOLO_ISOLATED_CAPABILITY_EXEC_TEST";
const ISOLATED_EXEC_TEST_FAILURE: &str = "isolated process capability exec test failed";
const SIGPIPE_DISPOSITION_CHANGED: &str = "isolated exec changed the parent SIGPIPE disposition";
const INHERITANCE_TEST_ROLE: &str = "inheritance";
const RESTORE_TEST_ROLE: &str = "restore";
const OWNERSHIP_TEST_ROLE: &str = "ownership";
const INHERITANCE_TEST: &str = "macos_process_capability::exec::tests::exec_inheritance_window_is_serialized_and_failure_restores_cloexec";
const RESTORE_TEST: &str = "macos_process_capability::exec::tests::restore_failure_closes_original_and_retains_only_a_cloexec_backup_under_gate";
const OWNERSHIP_TEST: &str = "macos_process_capability::exec::tests::concurrent_hook_installation_waits_for_the_current_owner";
const BLOCKING_PROBE_TIMEOUT: Duration = Duration::from_millis(25);
const CONCURRENCY_COMPLETION_TIMEOUT: Duration = Duration::from_secs(2);
const FCNTL_FAILURE: libc::c_int = -1;

#[test]
fn exec_inheritance_window_is_serialized_and_failure_restores_cloexec() {
    if run_isolated_unless_helper(INHERITANCE_TEST, INHERITANCE_TEST_ROLE) {
        return;
    }
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (_reader, mut writer) = super::super::test_support::transfer(issuer).unwrap();
    let pause = exec_test_hook::install();
    let exec_thread = std::thread::spawn(move || {
        let error = writer.exec_inheriting(&mut Command::new(MISSING_EXECUTABLE));
        (writer, error)
    });
    pause.wait_until_reached();

    let (acquired, observation) = channel();
    let gate_thread = std::thread::spawn(move || {
        let guard = crate::process::waitable_child_spawn_guard().unwrap();
        acquired.send(()).unwrap();
        drop(guard);
    });
    assert!(matches!(
        observation.recv_timeout(BLOCKING_PROBE_TIMEOUT),
        Err(RecvTimeoutError::Timeout)
    ));

    pause.release();
    let (writer, _error) = exec_thread.join().unwrap();
    observation
        .recv_timeout(CONCURRENCY_COMPLETION_TIMEOUT)
        .unwrap();
    gate_thread.join().unwrap();
    let flags = unsafe { libc::fcntl(writer.descriptor.as_raw_fd(), libc::F_GETFD) };
    assert_ne!(flags, FCNTL_FAILURE);
    assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
}

#[test]
fn restore_failure_closes_original_and_retains_only_a_cloexec_backup_under_gate() {
    if run_isolated_unless_helper(RESTORE_TEST, RESTORE_TEST_ROLE) {
        return;
    }
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (_reader, mut writer) = super::super::test_support::transfer(issuer).unwrap();
    let pause = exec_test_hook::install_restore_failure();
    let exec_thread = std::thread::spawn(move || {
        let error = writer.exec_inheriting(&mut Command::new(MISSING_EXECUTABLE));
        (writer, error)
    });
    pause.wait_until_reached();
    let (original, backup) = pause.descriptors();

    assert_eq!(
        unsafe { libc::fcntl(original, libc::F_GETFD) },
        FCNTL_FAILURE
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::EBADF)
    );
    let backup_flags = unsafe { libc::fcntl(backup, libc::F_GETFD) };
    assert_ne!(backup_flags, FCNTL_FAILURE);
    assert_ne!(backup_flags & libc::FD_CLOEXEC, libc::c_int::default());

    let (acquired, observation) = channel();
    let gate_thread = std::thread::spawn(move || {
        let guard = crate::process::waitable_child_spawn_guard().unwrap();
        acquired.send(()).unwrap();
        drop(guard);
    });
    assert!(matches!(
        observation.recv_timeout(BLOCKING_PROBE_TIMEOUT),
        Err(RecvTimeoutError::Timeout)
    ));

    pause.release();
    let (writer, _error) = exec_thread.join().unwrap();
    observation
        .recv_timeout(CONCURRENCY_COMPLETION_TIMEOUT)
        .unwrap();
    gate_thread.join().unwrap();
    assert_eq!(writer.descriptor.as_raw_fd(), backup);
    let flags = unsafe { libc::fcntl(writer.descriptor.as_raw_fd(), libc::F_GETFD) };
    assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
}

#[test]
fn concurrent_hook_installation_waits_for_the_current_owner() {
    if run_isolated_unless_helper(OWNERSHIP_TEST, OWNERSHIP_TEST_ROLE) {
        return;
    }
    let owner = exec_test_hook::install();
    assert!(exec_test_hook::installation_is_owned());
    let (attempting_sender, attempting_receiver) = channel();
    let (installed_sender, installed_receiver) = channel();
    let (release_sender, release_receiver) = channel();
    let waiter = std::thread::spawn(move || {
        attempting_sender.send(()).unwrap();
        let _waiting_owner = exec_test_hook::install_restore_failure();
        installed_sender.send(()).unwrap();
        release_receiver.recv().unwrap();
    });
    attempting_receiver.recv().unwrap();
    assert_eq!(
        installed_receiver.recv_timeout(BLOCKING_PROBE_TIMEOUT),
        Err(RecvTimeoutError::Timeout)
    );
    assert!(!exec_test_hook::fail_next_restore());
    drop(owner);
    installed_receiver
        .recv_timeout(CONCURRENCY_COMPLETION_TIMEOUT)
        .unwrap();
    release_sender.send(()).unwrap();
    waiter.join().unwrap();
}

fn run_isolated_unless_helper(test_name: &str, role: &str) -> bool {
    if std::env::var(ISOLATED_EXEC_TEST_ENVIRONMENT).as_deref() == Ok(role) {
        return false;
    }
    // A failed in-process Command::exec leaves SIGPIPE at SIG_DFL on macOS.
    let initial_sigpipe = sigpipe_handler();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            test_name,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
            TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(ISOLATED_EXEC_TEST_ENVIRONMENT, role);
    let mut child = spawn_guarded(&mut command);
    let status = child.wait().unwrap();
    assert!(status.success(), "{ISOLATED_EXEC_TEST_FAILURE}: {status}");
    assert_eq!(
        sigpipe_handler(),
        initial_sigpipe,
        "{SIGPIPE_DISPOSITION_CHANGED}"
    );
    true
}

fn spawn_guarded(command: &mut Command) -> Child {
    let _spawn_guard = crate::process::waitable_child_spawn_guard().unwrap();
    command.spawn().unwrap()
}

fn sigpipe_handler() -> libc::sighandler_t {
    let mut action = MaybeUninit::<libc::sigaction>::uninit();
    let result = unsafe { libc::sigaction(libc::SIGPIPE, std::ptr::null(), action.as_mut_ptr()) };
    assert_eq!(result, libc::c_int::default());
    unsafe { action.assume_init() }.sa_sigaction
}
