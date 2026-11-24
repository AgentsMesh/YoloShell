use super::*;
use crate::constants::{
    TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT,
    TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
};
use crate::process::MacosProcessCapabilityRelease;
use std::os::fd::{FromRawFd, OwnedFd};
use std::process::Command;

const PIPE_ENDPOINT_COUNT: usize = 2;
const PIPE_READER_INDEX: usize = 0;
const PIPE_WRITER_INDEX: usize = 1;
const MANAGER_TRANSFER_HELPER_ENV: &str = "YOLO_MANAGER_TRANSFER_HELPER";
const MANAGER_TRANSFER_HELPER_ROLE: &str = "run-manager-transfer-helper";
const MANAGER_TRANSFER_HELPER_TEST: &str =
    "worker::bootstrap::bootstrap_gate::descriptor::tests::manager_transfer_helper";

#[test]
fn manager_transfers_only_the_exact_child_writer() {
    let _spawn_guard = crate::process::waitable_child_spawn_guard().unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            MANAGER_TRANSFER_HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
            TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(MANAGER_TRANSFER_HELPER_ENV, MANAGER_TRANSFER_HELPER_ROLE)
        .status()
        .unwrap();
    assert!(status.success());
}

#[tokio::test]
async fn manager_transfer_helper() {
    if std::env::var(MANAGER_TRANSFER_HELPER_ENV).as_deref() != Ok(MANAGER_TRANSFER_HELPER_ROLE) {
        return;
    }
    let (manager, child) = UnixStream::pair().unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let expected = issuer.identity();
    let (sent, received) = tokio::join!(send(&manager, &issuer), receive(&child));
    sent.unwrap();
    let writer = received.unwrap();
    assert_eq!(writer.identity(), expected);
    let identity = writer.identity();
    let reader = issuer
        .commit_acknowledgement(identity.reader_handle(), identity.writer_handle())
        .unwrap();
    assert_eq!(reader.writer_handle_for_test(), expected.writer_handle());
    assert_eq!(
        reader.poll_release_for_test().unwrap(),
        MacosProcessCapabilityRelease::WriterPresent
    );
    drop(writer);
    assert_eq!(
        reader.poll_release_for_test().unwrap(),
        MacosProcessCapabilityRelease::Released
    );
}

#[tokio::test]
async fn wrong_direction_descriptor_is_rejected() {
    let (manager, child) = UnixStream::pair().unwrap();
    let mut descriptors = [libc::c_int::default(); PIPE_ENDPOINT_COUNT];
    assert_eq!(
        unsafe { libc::pipe(descriptors.as_mut_ptr()) },
        libc::c_int::default()
    );
    let reader = unsafe { OwnedFd::from_raw_fd(descriptors[PIPE_READER_INDEX]) };
    let _writer = unsafe { OwnedFd::from_raw_fd(descriptors[PIPE_WRITER_INDEX]) };
    let sent = send_once(
        manager.as_raw_fd(),
        reader.as_raw_fd(),
        crate::process::MACOS_PROCESS_CAPABILITY_TRANSFER_MARKER,
    );
    sent.unwrap();
    assert!(receive(&child).await.is_err());
}
