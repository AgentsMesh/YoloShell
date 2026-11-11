use super::super::*;
use crate::process::inventory_test_support::{TestProcess, entry};
use crate::process::{
    LiveProcessIdentity, MacosProcessCapabilityIssuer, MacosProcessCapabilityRelease,
    ProcessIdentity, ScopedWaitableChild,
};
use portable_pty::{MasterPty, PtyPair, PtySize, native_pty_system};
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::process::{Command, Stdio};
use std::time::Instant;

const TEST_CAPABILITY: &str = "released-root-pty-authority";
const PROCESS_EXECUTABLE: &str = "/bin/sleep";
const PROCESS_DURATION: &str = "30";
const TEST_SESSION_ID: u32 = 84_101;
const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;

pub(super) enum RootPtyBinding {
    Exact,
    Foreign,
}

pub(super) struct RootScanFixture {
    _target: Box<dyn MasterPty + Send>,
    _foreign: Option<Box<dyn MasterPty + Send>>,
    _root: ChildGuard,
    _ambient: TestProcess,
    snapshot: StableInventory,
    proof: PtySessionProof,
    root: ProcessIdentity,
    root_live: LiveProcessIdentity,
    ambient_lineage: u64,
}

impl RootScanFixture {
    pub(super) fn spawn(binding: RootPtyBinding) -> Self {
        let target = open_pty();
        let foreign = matches!(binding, RootPtyBinding::Foreign).then(open_pty);
        let holder = foreign.as_ref().unwrap_or(&target);
        let root = ChildGuard::spawn(open_slave(&holder.master.tty_name().unwrap()));
        let target = retain_master(target);
        let foreign = foreign.map(retain_master);
        let ambient = TestProcess::spawn();
        let root_entry = entry(root.identity, u32::default(), TEST_SESSION_ID);
        let ambient_entry = entry(ambient.identity, u32::default(), TEST_SESSION_ID);
        let root_live = LiveProcessIdentity::capture(root.identity)
            .unwrap()
            .unwrap();
        let ambient_lineage = ambient_entry.lineage_id();
        let stable = HashSet::from([root_entry.lineage_id(), ambient_lineage]);
        let snapshot = inventory(vec![root_entry, ambient_entry], stable);
        let proof = released_proof(target.as_ref());
        Self {
            _target: target,
            _foreign: foreign,
            _root: root,
            _ambient: ambient,
            snapshot,
            proof,
            root: root_entry.identity(),
            root_live,
            ambient_lineage,
        }
    }

    pub(super) fn root_live(&self) -> LiveProcessIdentity {
        self.root_live
    }

    pub(super) fn ambient_lineage(&self) -> u64 {
        self.ambient_lineage
    }

    pub(super) fn scan(&self, state: &ReleasedScanState) -> CapabilityScanResult {
        scan_escaped(
            &self.snapshot,
            &self.proof,
            state,
            self.root.start_time,
            self.root_live.lineage_id(),
            MacosGlobalScanMode::TransferableWriterReleased,
            true,
            &HashSet::from([self.root_live]),
            &HashSet::new(),
        )
        .unwrap()
    }
}

fn released_proof(master: &dyn MasterPty) -> PtySessionProof {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let spawn_guard = crate::process::waitable_child_spawn_guard().unwrap();
    let (reader, writer) = issuer.transfer_for_test().unwrap();
    let mut proof = PtySessionProof::capture_bound(master, TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(reader.clone()).unwrap();
    drop(writer);
    assert_eq!(
        reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::Released
    );
    drop(spawn_guard);
    proof
}

fn open_pty() -> PtyPair {
    native_pty_system()
        .openpty(PtySize {
            rows: TERMINAL_ROWS,
            cols: TERMINAL_COLUMNS,
            pixel_width: TERMINAL_PIXEL_SIZE,
            pixel_height: TERMINAL_PIXEL_SIZE,
        })
        .unwrap()
}

fn retain_master(pair: PtyPair) -> Box<dyn MasterPty + Send> {
    drop(pair.slave);
    pair.master
}

fn open_slave(path: &std::path::Path) -> std::fs::File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(path)
        .unwrap()
}

struct ChildGuard {
    _child: ScopedWaitableChild,
    identity: ProcessIdentity,
}

impl ChildGuard {
    fn spawn(input: std::fs::File) -> Self {
        let mut command = Command::new(PROCESS_EXECUTABLE);
        command
            .arg(PROCESS_DURATION)
            .stdin(Stdio::from(input))
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = ScopedWaitableChild::spawn_before_deadline(
            &mut command,
            Instant::now() + super::TEST_TIMEOUT,
        )
        .unwrap();
        let identity = ProcessIdentity::capture(child.process_id()).unwrap();
        Self {
            _child: child,
            identity,
        }
    }
}
