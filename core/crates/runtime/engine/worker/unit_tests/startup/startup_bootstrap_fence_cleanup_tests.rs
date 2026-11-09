use super::test_output;
use crate::process::{MacosProcessCapabilityIssuer, PtySessionProof};
use crate::worker::{
    BootstrapSpawnCommand, StartupCleanupScope, StartupPtyResources, UnreleasedBootstrapGate,
    spawn_tracked_pty_root,
};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use std::os::fd::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[path = "../../test_support/unit/startup/startup_bootstrap_fence_support.rs"]
mod support;
use support::*;

#[test]
fn bootstrap_fenced_child_helper() {
    let role = std::env::var(HELPER_ROLE_ENV).ok();
    if !matches!(
        role.as_deref(),
        Some(HELPER_ROLE) | Some(HELPER_HANGUP_ROLE)
    ) {
        return;
    }
    let release = PathBuf::from(std::env::var_os(HELPER_RELEASE_ENV).unwrap());
    let exiting = PathBuf::from(std::env::var_os(HELPER_EXITING_ENV).unwrap());
    let socket = PathBuf::from(std::env::var_os(HELPER_DESCRIPTOR_SOCKET_ENV).unwrap());
    let stream = UnixStream::connect(socket).unwrap();
    let writer = receive_descriptor(&stream);
    std::mem::forget(writer);
    while !release.exists() {
        std::thread::sleep(HELPER_POLL_INTERVAL);
    }
    std::fs::write(exiting, []).unwrap();
    if role.as_deref() == Some(HELPER_HANGUP_ROLE) {
        loop {
            std::thread::park();
        }
    }
}

#[tokio::test]
async fn pre_gate_child_exit_retires_tracked_pty_and_publishes_completion() {
    exercise_cleanup(false).await;
}

#[tokio::test]
async fn pre_gate_master_closure_reaps_the_live_helper_before_profile_cleanup() {
    let signal = exercise_cleanup(true).await;
    assert!(matches!(signal, Some(libc::SIGHUP) | Some(libc::SIGKILL)));
}

#[test]
fn profile_cleanup_is_limited_to_waited_pty_hangup_or_kill() {
    assert!(interrupted_profile(Some(libc::SIGHUP)));
    assert!(interrupted_profile(Some(libc::SIGKILL)));
    for signal in [
        None,
        Some(libc::SIGTERM),
        Some(libc::SIGABRT),
        Some(libc::SIGSEGV),
    ] {
        assert!(!interrupted_profile(signal));
    }
}

async fn exercise_cleanup(hold_until_hangup: bool) -> Option<i32> {
    let directory = tempfile::tempdir().unwrap();
    let release = directory.path().join(RELEASE_FILE);
    let exiting = directory.path().join(EXITING_FILE);
    let descriptor_socket = directory.path().join(DESCRIPTOR_SOCKET_FILE);
    let descriptor_listener = UnixListener::bind(&descriptor_socket).unwrap();
    descriptor_listener.set_nonblocking(true).unwrap();
    let mut cleanup_scope = StartupCleanupScope::new().unwrap();
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let capability = Uuid::new_v4().to_string();
    let mut proof =
        PtySessionProof::capture_bootstrap_bound(pair.master.as_ref(), capability.clone()).unwrap();
    proof.bind_runtime(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    let (reader, writer) = MacosProcessCapabilityIssuer::create()
        .unwrap()
        .into_staged_for_test()
        .unwrap();
    let output = test_output::capture_test_output(
        pair.master.as_ref(),
        &directory.path().join(JOURNAL_MARKER),
    );
    let mut command = bootstrap_helper_command(&release, &exiting, &descriptor_socket, &capability);
    if hold_until_hangup {
        command.command.env(HELPER_ROLE_ENV, HELPER_HANGUP_ROLE);
    }
    let mut child = spawn_tracked_pty_root(
        pair.slave.as_ref(),
        command,
        proof,
        output,
        &mut cleanup_scope,
    )
    .unwrap();
    let descriptor_stream = accept_descriptor_stream(&descriptor_listener);
    let child_pid = child.process_id().unwrap();
    crate::worker::send_bootstrap_writer_once_for_test(
        descriptor_stream.as_raw_fd(),
        writer.descriptor(),
    )
    .unwrap();
    child.install_process_capability(reader).unwrap();
    drop(writer);
    let completion = child
        .fault_reaper
        .as_mut()
        .unwrap()
        .observe_startup_completion();
    let resources = StartupPtyResources::from_guard(pair.master, child);
    drop(pair.slave);

    std::fs::write(&release, []).unwrap();
    wait_for_marker(&exiting);
    drop(resources);

    let outcome = completion
        .receive_timeout(CLEANUP_TIMEOUT)
        .expect(CLEANUP_TIMEOUT_DETAIL);
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::Waited
    );
    let status = outcome.status.unwrap();
    let signal = status.signal();
    if interrupted_profile(signal) {
        yolo_coverage_test_support::discard_aborted_process_profile(child_pid).unwrap();
    }
    cleanup_scope.settle().await.unwrap();
    signal
}
