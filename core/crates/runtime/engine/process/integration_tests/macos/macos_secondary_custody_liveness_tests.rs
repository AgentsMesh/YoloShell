use super::*;
use crate::process::{LiveProcessIdentity, MacosProcessCapabilityIssuer, PtySessionProof};
use portable_pty::{PtySize, native_pty_system};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

const TEST_CAPABILITY: &str = "macos-secondary-custody-liveness";
const TEST_TERMINAL_ROWS: u16 = 24;
const TEST_TERMINAL_COLUMNS: u16 = 80;
const TEST_TERMINAL_PIXEL_SIZE: u16 = 0;
const RELEASE_POLL_INTERVAL: Duration = Duration::from_millis(1);
const RELEASE_TIMEOUT: Duration = crate::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT;
const PREMATURE_RELEASE_MISCLASSIFIED: &str =
    "transferred custody accepted a capability release while its root was live";
const PREMATURE_RELEASE_DETAIL: &str = "process capability writer released before the root exited";
const EXITED_ROOT_COMMAND: &str = "/bin/sleep";
const EXITED_ROOT_DURATION: &str = "30";

#[test]
fn transferred_reader_preserves_premature_release_poison() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    let (reader, writer) = MacosProcessCapabilityIssuer::create()
        .unwrap()
        .into_staged()
        .unwrap();
    proof.install_process_capability(reader).unwrap();
    let root = ProcessIdentity::capture(std::process::id()).unwrap();
    let runtime = runtime(root);
    let _alias_release = proof
        .bind_secondary_custody_alias_for_test(&runtime)
        .unwrap();
    let export = MacosSecondaryCustodyExport::capture(&proof, &runtime).unwrap();
    let digest = export.recovery_identity_sha256();
    let descriptor_set = export.descriptor_set();
    let custody = MacosSecondaryCustody::from_transferred(
        export.into_parts(),
        &runtime,
        digest,
        descriptor_set,
        CustodyProfile::ActiveRunning,
    )
    .unwrap();

    drop(writer);
    let deadline = Instant::now() + RELEASE_TIMEOUT;
    loop {
        match custody.closure_settled() {
            Err(error) => {
                assert!(error.to_string().contains(PREMATURE_RELEASE_DETAIL));
                return;
            }
            Ok(false) if Instant::now() < deadline => std::thread::sleep(RELEASE_POLL_INTERVAL),
            Ok(_) => panic!("{PREMATURE_RELEASE_MISCLASSIFIED}"),
        }
    }
}

#[test]
fn transferred_custody_accepts_an_exact_root_that_exited_after_export() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    let (reader, writer) = MacosProcessCapabilityIssuer::create()
        .unwrap()
        .into_staged()
        .unwrap();
    proof.install_process_capability(reader).unwrap();
    let mut command = Command::new(EXITED_ROOT_COMMAND);
    command.arg(EXITED_ROOT_DURATION);
    writer.configure_exec_inheritance(&mut command);
    let child = crate::process::spawn_waitable_test_child(command);
    let mut child = ChildGuard(Some(child));
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let runtime = runtime(root);
    let alias_release = proof
        .bind_secondary_custody_alias_for_test(&runtime)
        .unwrap();
    let export = MacosSecondaryCustodyExport::capture(&proof, &runtime).unwrap();
    let digest = export.recovery_identity_sha256();
    let descriptor_set = export.descriptor_set();
    drop((writer, proof, pair, alias_release));
    child.terminate();
    assert!(LiveProcessIdentity::capture(root).unwrap().is_none());

    let custody = MacosSecondaryCustody::from_transferred(
        export.into_parts(),
        &runtime,
        digest,
        descriptor_set,
        CustodyProfile::ActiveTerminalReadOnly,
    )
    .unwrap();

    assert!(custody.closure_settled().unwrap());
}

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn id(&self) -> u32 {
        self.0.as_ref().unwrap().id()
    }

    fn terminate(&mut self) {
        let mut child = self.0.take().unwrap();
        child.kill().unwrap();
        child.wait().unwrap();
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

fn runtime(identity: ProcessIdentity) -> CustodyRuntimeIdentity {
    CustodyRuntimeIdentity {
        terminal_id: uuid::Uuid::new_v4(),
        runtime_epoch: uuid::Uuid::new_v4(),
        worker_id: uuid::Uuid::new_v4(),
        worker_pid: identity.pid,
        worker_start_time: identity.start_time,
        child_pid: identity.pid,
        child_start_time: identity.start_time,
        boot_id: crate::process::boot_id(),
    }
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TEST_TERMINAL_ROWS,
        cols: TEST_TERMINAL_COLUMNS,
        pixel_width: TEST_TERMINAL_PIXEL_SIZE,
        pixel_height: TEST_TERMINAL_PIXEL_SIZE,
    }
}
