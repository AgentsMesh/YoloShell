use super::*;
use crate::process::{MacosProcessCapabilityIssuer, PtySessionProof};
use portable_pty::{PtySize, native_pty_system};
use std::os::unix::process::CommandExt;
use std::time::Duration;

#[path = "macos_secondary_custody_settlement_test_helper.rs"]
mod tree_helper;

const TEST_CAPABILITY: &str = "macos-secondary-custody-settlement-test";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const TEST_CHILD_COMMAND: &str = "/bin/sleep";
const TEST_CHILD_DURATION_SECONDS: &str = "30";
const SETTLEMENT_POLL_INTERVAL: Duration = Duration::from_millis(10);
const FCNTL_FAILURE: libc::c_int = -1;

#[test]
fn active_settlement_kills_the_exact_capability_child_and_proves_release() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap();
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    let (reader, writer) = MacosProcessCapabilityIssuer::create()
        .unwrap()
        .into_staged()
        .unwrap();
    proof.install_process_capability(reader).unwrap();
    let writer_descriptor = writer.descriptor();
    let descriptor_flags = descriptor_flags(writer_descriptor);
    let mut command = std::process::Command::new(TEST_CHILD_COMMAND);
    command.arg(TEST_CHILD_DURATION_SECONDS);
    unsafe {
        command.pre_exec(move || {
            set_descriptor_flags_io(writer_descriptor, descriptor_flags & !libc::FD_CLOEXEC)
        });
    }
    let child = crate::process::spawn_waitable_test_child(command);
    let mut child = ChildGuard(Some(child));
    let child_identity = crate::process::ProcessIdentity::capture(child.id()).unwrap();
    let runtime = runtime(child_identity);
    let alias_release = proof
        .bind_secondary_custody_alias_for_test(&runtime)
        .unwrap();
    let exported = MacosSecondaryCustodyExport::capture(&proof, &runtime).unwrap();
    let digest = exported.recovery_identity_sha256();
    let descriptor_set = exported.descriptor_set();
    let custody = MacosSecondaryCustody::from_transferred(
        exported.into_parts(),
        &runtime,
        digest,
        descriptor_set,
        CustodyProfile::ActiveRunning,
    )
    .unwrap();
    drop((writer, proof, pair, alias_release));

    assert!(!custody.request_active_settlement().unwrap());
    let status = child.wait();
    assert!(!status.success());
    let deadline = std::time::Instant::now()
        .checked_add(crate::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT)
        .unwrap();
    while !custody.closure_settled().unwrap() && std::time::Instant::now() < deadline {
        std::thread::sleep(SETTLEMENT_POLL_INTERVAL);
    }
    assert!(custody.closure_settled().unwrap());
}

#[test]
fn active_settlement_kills_an_escaped_inherited_writer_tree() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap();
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    let (reader, writer) = MacosProcessCapabilityIssuer::create()
        .unwrap()
        .into_staged()
        .unwrap();
    proof.install_process_capability(reader).unwrap();
    let writer_descriptor = writer.descriptor();
    let descriptor_flags = descriptor_flags(writer_descriptor);
    let mut tree = tree_helper::InheritedWriterTree::spawn(writer_descriptor, descriptor_flags);
    let root = tree.root_identity;
    let leaf = tree.wait_for_leaf();
    let runtime = runtime(root);
    let alias_release = proof
        .bind_secondary_custody_alias_for_test(&runtime)
        .unwrap();
    let exported = MacosSecondaryCustodyExport::capture(&proof, &runtime).unwrap();
    let digest = exported.recovery_identity_sha256();
    let descriptor_set = exported.descriptor_set();
    drop((writer, proof, pair, alias_release));
    tree.terminate_root_preserving_leaf();
    assert!(!root.matches());
    assert!(leaf.matches());
    let custody = MacosSecondaryCustody::from_transferred(
        exported.into_parts(),
        &runtime,
        digest,
        descriptor_set,
        CustodyProfile::ActiveRunning,
    )
    .unwrap();

    assert!(!custody.closure_settled().unwrap());
    assert!(!custody.request_active_settlement().unwrap());
    tree.assert_tree_terminated();
    wait_for_closure(&custody);
    assert!(custody.request_active_settlement().unwrap());
    assert!(!root.matches());
    assert!(!leaf.matches());
}

#[test]
fn inherited_writer_tree_helper() {
    tree_helper::run_role();
}

fn wait_for_closure(custody: &MacosSecondaryCustody) {
    let deadline = std::time::Instant::now()
        .checked_add(crate::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT)
        .unwrap();
    while !custody.closure_settled().unwrap() && std::time::Instant::now() < deadline {
        std::thread::sleep(SETTLEMENT_POLL_INTERVAL);
    }
    assert!(custody.closure_settled().unwrap());
}

fn runtime(child: crate::process::ProcessIdentity) -> CustodyRuntimeIdentity {
    let worker = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    CustodyRuntimeIdentity {
        terminal_id: uuid::Uuid::new_v4(),
        runtime_epoch: uuid::Uuid::new_v4(),
        worker_id: uuid::Uuid::new_v4(),
        worker_pid: worker.pid,
        worker_start_time: worker.start_time,
        child_pid: child.pid,
        child_start_time: child.start_time,
        boot_id: crate::process::boot_id(),
    }
}

fn descriptor_flags(descriptor: libc::c_int) -> libc::c_int {
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    assert_ne!(flags, FCNTL_FAILURE);
    flags
}

fn set_descriptor_flags_io(descriptor: libc::c_int, flags: libc::c_int) -> std::io::Result<()> {
    match unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags) } {
        FCNTL_FAILURE => Err(std::io::Error::last_os_error()),
        _ => Ok(()),
    }
}

struct ChildGuard(Option<std::process::Child>);

impl ChildGuard {
    fn id(&self) -> u32 {
        self.0.as_ref().unwrap().id()
    }

    fn wait(&mut self) -> std::process::ExitStatus {
        self.0.take().unwrap().wait().unwrap()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
