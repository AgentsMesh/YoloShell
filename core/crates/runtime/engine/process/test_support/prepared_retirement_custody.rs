use crate::process::{
    MacosProcessCapabilityIssuer, MacosSecondaryCustody, MacosSecondaryCustodyExport,
    ProcessIdentity, PtySessionProof,
};
use portable_pty::{PtySize, native_pty_system};
use std::os::unix::process::CommandExt;
use std::time::{Duration, Instant};
use yolo_pty_custody::{CustodyDescriptorSet, CustodyProfile, CustodyRuntimeIdentity};

const CAPABILITY_LABEL: &str = "prepared-retirement-publisher-test";
const CHILD_COMMAND: &str = "/bin/sleep";
const CHILD_DURATION_SECONDS: &str = "30";
const CLOSURE_POLL_INTERVAL: Duration = Duration::from_millis(10);
const FCNTL_FAILURE: libc::c_int = -1;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;
const TERMINAL_ROWS: u16 = 24;
const WORKER_COMMAND: &str = "/usr/bin/true";

#[doc(hidden)]
pub struct PreparedRetirementCustodyFixture {
    pub custody: MacosSecondaryCustody,
    pub runtime: CustodyRuntimeIdentity,
    pub descriptor_set: CustodyDescriptorSet,
    pub digest: crate::process::sha256_digest::Sha256Digest,
}

#[doc(hidden)]
pub fn prepared_retirement_custody_fixture() -> PreparedRetirementCustodyFixture {
    let worker = reaped_worker_identity();
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: TERMINAL_ROWS,
            cols: TERMINAL_COLUMNS,
            pixel_width: TERMINAL_PIXEL_SIZE,
            pixel_height: TERMINAL_PIXEL_SIZE,
        })
        .unwrap();
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), CAPABILITY_LABEL.to_owned()).unwrap();
    let (reader, writer) = MacosProcessCapabilityIssuer::create()
        .unwrap()
        .into_staged()
        .unwrap();
    proof.install_process_capability(reader).unwrap();
    let descriptor = writer.descriptor();
    let flags = descriptor_flags(descriptor);
    let mut command = std::process::Command::new(CHILD_COMMAND);
    command.arg(CHILD_DURATION_SECONDS);
    unsafe {
        command.pre_exec(move || set_flags(descriptor, flags & !libc::FD_CLOEXEC));
    }
    let mut child = ChildGuard::new(crate::process::spawn_waitable_test_child(command));
    let child_identity = ProcessIdentity::capture(child.id()).unwrap();
    let runtime = runtime(worker, child_identity);
    let alias = proof
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
    drop((writer, proof, pair, alias));
    let _ = custody.request_active_settlement().unwrap();
    assert!(!child.wait().success());
    custody.confirm_root_exit().unwrap();
    wait_for_closure(&custody);
    PreparedRetirementCustodyFixture {
        custody,
        runtime,
        descriptor_set,
        digest,
    }
}

fn reaped_worker_identity() -> ProcessIdentity {
    let mut command = std::process::Command::new(WORKER_COMMAND);
    let mut child = crate::process::spawn_waitable_test_child(&mut command);
    let identity = ProcessIdentity::capture(child.id()).unwrap();
    assert!(child.wait().unwrap().success());
    identity
}

fn runtime(worker: ProcessIdentity, child: ProcessIdentity) -> CustodyRuntimeIdentity {
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

fn wait_for_closure(custody: &MacosSecondaryCustody) {
    let deadline =
        Instant::now() + crate::process::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT;
    while !custody.closure_settled().unwrap() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(CLOSURE_POLL_INTERVAL);
    }
}

fn descriptor_flags(descriptor: libc::c_int) -> libc::c_int {
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    assert_ne!(flags, FCNTL_FAILURE);
    flags
}

fn set_flags(descriptor: libc::c_int, flags: libc::c_int) -> std::io::Result<()> {
    match unsafe { libc::fcntl(descriptor, libc::F_SETFD, flags) } {
        FCNTL_FAILURE => Err(std::io::Error::last_os_error()),
        _ => Ok(()),
    }
}

struct ChildGuard(Option<std::process::Child>);

impl ChildGuard {
    fn new(child: std::process::Child) -> Self {
        Self(Some(child))
    }

    fn id(&self) -> u32 {
        self.0.as_ref().unwrap().id()
    }

    fn wait(&mut self) -> std::process::ExitStatus {
        self.0.take().unwrap().wait().unwrap()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(child) = self.0.take() {
            crate::process::terminate_detached_child(child)
                .expect("prepared retirement fixture must retain exact child custody");
        }
    }
}
