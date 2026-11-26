use super::captured_tree_macos_wait::wait_for_path;
use super::{
    CapturedProcessTree, MacosProcessCapabilityChildWriter, MacosProcessCapabilityIssuer,
    ProcessIdentity,
};
use crate::runtime_constants::{
    TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT,
    TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const HELPER_TEST: &str =
    "pty_session_descriptor_capability_reuse_tests::descriptor_capability_helper";
const HELPER_ROLE_ENV: &str = "YOLO_DESCRIPTOR_REUSE_HELPER_ROLE";
const HELPER_DESCRIPTOR_ENV: &str = "YOLO_DESCRIPTOR_REUSE_HELPER_FD";
const HELPER_READY_ENV: &str = "YOLO_DESCRIPTOR_REUSE_HELPER_READY";
const HELPER_TRIGGER_ENV: &str = "YOLO_DESCRIPTOR_REUSE_HELPER_TRIGGER";
const HELPER_HANDLES_ENV: &str = "YOLO_DESCRIPTOR_REUSE_HELPER_HANDLES";
const TARGET_ROLE: &str = "target";
const ALLOCATOR_ROLE: &str = "allocator";
const TARGET_READY_FILE: &str = "target.ready";
const TARGET_CLOSED_FILE: &str = "target.closed";
const TARGET_TRIGGER_FILE: &str = "target.trigger";
const ALLOCATOR_READY_FILE: &str = "allocator.ready";
const ALLOCATOR_HANDLES_FILE: &str = "allocator.handles";
const TEST_CAPABILITY: &str = "retained-descriptor-reuse-capability";
const HANDLE_SEPARATOR: &str = "\n";
const EMPTY_PATH: &str = "";
const EMPTY_MARKER: &[u8] = b"";
const UNRELATED_ALLOCATION_ATTEMPTS: usize = 64;
const HELPER_LIFETIME: Duration = Duration::from_secs(30);
const CLOSE_SUCCESS: libc::c_int = 0;

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn retained_authority_excludes_a_post_close_unrelated_allocator() {
    let directory = tempfile::tempdir().unwrap();
    let target_ready = directory.path().join(TARGET_READY_FILE);
    let target_closed = directory.path().join(TARGET_CLOSED_FILE);
    let trigger = directory.path().join(TARGET_TRIGGER_FILE);
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (authority, writer) = issuer.into_staged().unwrap();
    let expected = writer.identity().writer_handle();
    let mut target = ChildGuard(spawn_helper(
        TARGET_ROLE,
        &target_ready,
        &target_closed,
        &trigger,
        Some(&writer),
    ));
    drop(writer);
    wait_for_path(&target_ready);

    let root = ProcessIdentity::capture(target.0.id()).unwrap();
    let mut tree =
        CapturedProcessTree::capture_with_discovery_internal(root, TEST_CAPABILITY.to_owned())
            .unwrap();
    tree.install_process_capability(authority).unwrap();
    tree.refresh().unwrap();
    std::fs::write(&trigger, EMPTY_MARKER).unwrap();
    wait_for_path(&target_closed);

    let allocator_ready = directory.path().join(ALLOCATOR_READY_FILE);
    let handles = directory.path().join(ALLOCATOR_HANDLES_FILE);
    let mut allocator = ChildGuard(spawn_helper(
        ALLOCATOR_ROLE,
        &allocator_ready,
        &handles,
        Path::new(EMPTY_PATH),
        None,
    ));
    wait_for_path(&allocator_ready);
    assert!(
        read_handles(&handles)
            .iter()
            .all(|handle| *handle != expected)
    );

    tree.refresh().unwrap();
    let allocator_identity = ProcessIdentity::capture(allocator.0.id()).unwrap();
    assert!(!tree.identities().contains(&allocator_identity));
    let signalled = tree.terminate_captured(libc::SIGKILL).unwrap();
    assert!(signalled.contains(&root));
    assert!(!signalled.contains(&allocator_identity));
    assert!(allocator.0.try_wait().unwrap().is_none());
    target.0.wait().unwrap();
    allocator.0.kill().unwrap();
    allocator.0.wait().unwrap();
}

#[test]
fn descriptor_capability_helper() {
    match std::env::var(HELPER_ROLE_ENV).as_deref() {
        Ok(TARGET_ROLE) => run_target(),
        Ok(ALLOCATOR_ROLE) => run_allocator(),
        _ => {}
    }
}

fn run_target() {
    let ready = required_path(HELPER_READY_ENV);
    std::fs::write(ready, EMPTY_MARKER).unwrap();
    wait_for_path(&required_path(HELPER_TRIGGER_ENV));
    let descriptor = std::env::var(HELPER_DESCRIPTOR_ENV)
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(unsafe { libc::close(descriptor) }, CLOSE_SUCCESS);
    std::fs::write(required_path(HELPER_HANDLES_ENV), EMPTY_MARKER).unwrap();
    std::thread::sleep(HELPER_LIFETIME);
}

fn run_allocator() {
    let capabilities = (0..UNRELATED_ALLOCATION_ATTEMPTS)
        .map(|_| {
            MacosProcessCapabilityIssuer::create()
                .unwrap()
                .into_staged()
                .unwrap()
        })
        .collect::<Vec<_>>();
    let handles = capabilities
        .iter()
        .map(|(_reader, writer)| writer.identity().writer_handle().to_string())
        .collect::<Vec<_>>()
        .join(HANDLE_SEPARATOR);
    std::fs::write(required_path(HELPER_HANDLES_ENV), handles).unwrap();
    std::fs::write(required_path(HELPER_READY_ENV), EMPTY_MARKER).unwrap();
    std::thread::sleep(HELPER_LIFETIME);
}

fn spawn_helper(
    role: &str,
    ready: &Path,
    handles: &Path,
    trigger: &Path,
    authority: Option<&MacosProcessCapabilityChildWriter>,
) -> Child {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            HELPER_TEST,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
            TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(HELPER_ROLE_ENV, role)
        .env(HELPER_READY_ENV, ready)
        .env(HELPER_HANDLES_ENV, handles)
        .env(HELPER_TRIGGER_ENV, trigger)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    super::configure_detached_command(&mut command);
    if let Some(authority) = authority {
        command.env(HELPER_DESCRIPTOR_ENV, authority.descriptor().to_string());
        authority.configure_exec_inheritance(&mut command);
    }
    super::spawn_waitable_test_child(command)
}

fn read_handles(path: &Path) -> Vec<u64> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| line.parse().unwrap())
        .collect()
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}
