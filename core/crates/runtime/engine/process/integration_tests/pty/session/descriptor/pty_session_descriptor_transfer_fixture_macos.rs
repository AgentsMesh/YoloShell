use super::*;
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT, wait_for_path};
use crate::process::inventory_test_support::entry;
use crate::process::{
    LiveProcessIdentity, MacosProcessCapabilityIssuer, MacosProcessCapabilityManagerReader,
    MacosProcessCapabilityRelease, ProcessIdentity,
};
use std::os::unix::net::UnixStream;
use std::{path::PathBuf, sync::Arc, time::Instant};

pub(super) const TEST_CAPABILITY: &str = "descriptor-transfer-process-test";
const HELPER_TEST: &str = "pty_session_proof::macos_released::tests::descriptor_transfer_process_tests::descriptor_transfer_helper";
const HELPER_ROLE_ENV: &str = "YOLO_DESCRIPTOR_TRANSFER_ROLE";
const HELPER_READY_ENV: &str = "YOLO_DESCRIPTOR_TRANSFER_READY";
const HELPER_TRIGGER_ENV: &str = "YOLO_DESCRIPTOR_TRANSFER_TRIGGER";
const HELPER_COMPLETE_ENV: &str = "YOLO_DESCRIPTOR_TRANSFER_COMPLETE";
const HELPER_WRITER_ENV: &str = "YOLO_DESCRIPTOR_TRANSFER_WRITER";
const RECEIVER_ROLE: &str = "receiver";
const SENDER_ROLE: &str = "sender";
const RECEIVER_READY_FILE: &str = "receiver.ready";
const RECEIVER_TRIGGER_FILE: &str = "receiver.release";
const RECEIVER_COMPLETE_FILE: &str = "receiver.received";
const SENDER_READY_FILE: &str = "sender.ready";
const SENDER_TRIGGER_FILE: &str = "sender.transfer";
const SENDER_COMPLETE_FILE: &str = "sender.closed";
const TEST_SESSION_ID: u32 = 73_001;
const ROOT_EXITED: bool = true;
const SCAN_ESCAPED_PROCESSES: bool = false;
const RELEASE_MISSING_DETAIL: &str = "transferred writer did not release";
const ORDER_DETAIL: &str = "descriptor receiver must predate the capability root";
const EMPTY_MARKER: &[u8] = b"";
#[path = "../../../../test_support/pty_session/pty_session_descriptor_transfer_candidate_macos.rs"]
mod fixture_candidate;
#[path = "pty_session_descriptor_transfer_helper_macos.rs"]
mod helper;
#[path = "../../../../test_support/pty_session/pty_session_descriptor_transfer_spawn_macos.rs"]
mod spawn;
use self::fixture_candidate::released_candidate;
use spawn::{ChildGuard, helper_command};

pub(super) struct DescriptorTransferFixture {
    _directory: tempfile::TempDir,
    receiver: ChildGuard,
    root: ChildGuard,
    receiver_identity: ProcessIdentity,
    root_identity: ProcessIdentity,
    root_live: LiveProcessIdentity,
    reader: Arc<MacosProcessCapabilityManagerReader>,
    proof: PtySessionProof,
    receiver_trigger: PathBuf,
    receiver_complete: PathBuf,
    sender_trigger: PathBuf,
    sender_complete: PathBuf,
}

impl DescriptorTransferFixture {
    pub(super) fn launch() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let receiver_ready = directory.path().join(RECEIVER_READY_FILE);
        let receiver_trigger = directory.path().join(RECEIVER_TRIGGER_FILE);
        let receiver_complete = directory.path().join(RECEIVER_COMPLETE_FILE);
        let sender_ready = directory.path().join(SENDER_READY_FILE);
        let sender_trigger = directory.path().join(SENDER_TRIGGER_FILE);
        let sender_complete = directory.path().join(SENDER_COMPLETE_FILE);
        let (receiver_socket, sender_socket) = UnixStream::pair().unwrap();
        let receiver = ChildGuard::spawn(helper_command(
            RECEIVER_ROLE,
            &receiver_ready,
            &receiver_trigger,
            &receiver_complete,
            receiver_socket.into(),
        ));
        wait_for_path(&receiver_ready);
        let receiver_identity = ProcessIdentity::capture(receiver.id()).unwrap();
        let issuer = MacosProcessCapabilityIssuer::create().unwrap();
        let (reader, writer) = issuer.into_staged().unwrap();
        let mut root_command = helper_command(
            SENDER_ROLE,
            &sender_ready,
            &sender_trigger,
            &sender_complete,
            sender_socket.into(),
        );
        root_command.env(HELPER_WRITER_ENV, writer.descriptor().to_string());
        writer.configure_exec_inheritance(&mut root_command);
        let root = ChildGuard::spawn(root_command);
        let root_identity = ProcessIdentity::capture(root.id()).unwrap();
        let receiver_predates_root = receiver_identity.start_time < root_identity.start_time;
        assert!(receiver_predates_root, "{ORDER_DETAIL}");
        let root_live = LiveProcessIdentity::capture(root_identity)
            .unwrap()
            .unwrap();
        reader.verify_writer_holder(&root_live).unwrap();
        reader.arm_liveness(root_live).unwrap();
        drop(writer);
        wait_for_path(&sender_ready);
        let proof = proof_with_reader(Arc::clone(&reader));
        Self {
            _directory: directory,
            receiver,
            root,
            receiver_identity,
            root_identity,
            root_live,
            reader,
            proof,
            receiver_trigger,
            receiver_complete,
            sender_trigger,
            sender_complete,
        }
    }

    pub(super) fn transfer_and_exit_root(&mut self) {
        std::fs::write(&self.sender_trigger, EMPTY_MARKER).unwrap();
        wait_for_path(&self.sender_complete);
        wait_for_path(&self.receiver_complete);
        self.root.wait();
        self.reader.confirm_root_exit(&self.root_live).unwrap();
    }

    pub(super) fn release_receiver(&mut self) {
        std::fs::write(&self.receiver_trigger, EMPTY_MARKER).unwrap();
        self.receiver.wait();
    }

    pub(super) fn assert_writer_present(&self) {
        assert_eq!(
            self.reader.poll_release().unwrap(),
            MacosProcessCapabilityRelease::WriterPresent
        );
    }

    pub(super) fn wait_until_released(&self) {
        let deadline = Instant::now() + TEST_TIMEOUT;
        loop {
            match self.reader.poll_release().unwrap() {
                MacosProcessCapabilityRelease::Released => return,
                MacosProcessCapabilityRelease::WriterPresent => {}
            }
            assert!(Instant::now() < deadline, "{RELEASE_MISSING_DETAIL}");
            std::thread::sleep(POLL_INTERVAL);
        }
    }

    pub(super) fn proof(&self) -> &PtySessionProof {
        &self.proof
    }

    pub(super) fn root_lineage(&self) -> u64 {
        self.root_live.lineage_id()
    }

    pub(super) fn receiver_lineage(&self) -> u64 {
        self.receiver_candidate().live.lineage_id()
    }

    pub(super) fn receiver_candidate(&self) -> ReleasedCandidate {
        released_candidate(self.receiver_identity, false)
    }
    pub(super) fn root_candidate(&self) -> ReleasedCandidate {
        released_candidate(self.root_identity, true)
    }

    pub(super) fn receiver_inventory(&self) -> StableInventory {
        let receiver = entry(self.receiver_identity, std::process::id(), TEST_SESSION_ID);
        inventory(vec![receiver], HashSet::from([receiver.lineage_id()]))
    }

    pub(super) fn scan(
        &self,
        inventory: &StableInventory,
        state: &PtyCapabilityScanState,
    ) -> CapabilityScanResult {
        inventory
            .macos_capability_candidates(
                &self.proof,
                state,
                &Default::default(),
                &Default::default(),
                self.root_identity.start_time,
                self.root_lineage(),
                ROOT_EXITED,
                SCAN_ESCAPED_PROCESSES,
            )
            .unwrap()
    }
}

pub(super) fn run_helper() {
    helper::run();
}
