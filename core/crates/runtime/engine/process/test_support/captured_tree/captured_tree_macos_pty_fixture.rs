pub use super::captured_tree_macos_support::LineageFixture;
use super::{
    CapturedProcessTree, LiveProcessIdentity, MacosProcessCapabilityIssuer,
    PTY_SESSION_CAPABILITY_ENV, PtySessionProof,
};
use crate::process::inventory_snapshot::{
    InventoryEntry, TestInventoryGuard, install_repeating_test_inventory,
};
use crate::process::inventory_test_support::{NO_PARENT_PID, entry};
use portable_pty::{PtyPair, PtySize, native_pty_system};

const TERMINAL_ROWS: u16 = 24;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_PIXEL_SIZE: u16 = 0;
const TREE_ALREADY_TAKEN_DETAIL: &str = "PTY lineage tree was already taken";

pub struct PtyLineageFixture {
    lineage: LineageFixture,
    tree: Option<CapturedProcessTree>,
    inventory: TestInventoryGuard,
    root_lineage_id: u64,
    _pty: PtyPair,
}

impl PtyLineageFixture {
    pub fn spawn(capability: &str) -> Self {
        let pty = native_pty_system().openpty(pty_size()).unwrap();
        let issuer = MacosProcessCapabilityIssuer::create().unwrap();
        let (reader, writer) = issuer.transfer_for_test().unwrap();
        let lineage = LineageFixture::spawn_configured(|command| {
            command.env(PTY_SESSION_CAPABILITY_ENV, capability);
            writer.configure_exec_inheritance(command);
        });
        let root = lineage.root_identity();
        let root_live = LiveProcessIdentity::capture(root).unwrap().unwrap();
        reader.verify_writer_holder(&root_live).unwrap();
        drop(writer);
        let root_entry = entry(root, NO_PARENT_PID, root.pid);
        let root_lineage_id = root_entry.lineage_id();
        let inventory = install_repeating_test_inventory(vec![root_entry]);
        let mut proof =
            PtySessionProof::capture_bound(pty.master.as_ref(), capability.to_owned()).unwrap();
        proof.install_process_capability(reader).unwrap();
        let tree = CapturedProcessTree::capture_pty(root, proof).unwrap();
        Self {
            lineage,
            tree: Some(tree),
            inventory,
            root_lineage_id,
            _pty: pty,
        }
    }

    pub fn take_tree(&mut self) -> CapturedProcessTree {
        self.tree.take().expect(TREE_ALREADY_TAKEN_DETAIL)
    }

    pub fn root_identity(&self) -> super::ProcessIdentity {
        self.lineage.root_identity()
    }

    pub fn release_and_capture_leaf(&mut self) -> super::ProcessIdentity {
        let leaf = self.lineage.release_and_capture_leaf();
        self.publish_leaf(leaf);
        leaf
    }

    pub fn wait_for_root_exit_and_leaf_reparent(&self) {
        self.lineage.wait_for_root_exit_and_leaf_reparent();
    }

    pub fn release_and_observe_leaf(&mut self) -> super::ProcessIdentity {
        let leaf = self.lineage.release_and_observe_leaf();
        self.publish_leaf(leaf);
        leaf
    }

    fn publish_leaf(&mut self, leaf: super::ProcessIdentity) {
        let leaf_entry = leaf_entry_with_parent_unique_id(
            leaf,
            self.lineage.root_identity().pid,
            leaf.pid,
            self.root_lineage_id,
        );
        self.inventory.replace_repeating(vec![leaf_entry]);
    }

    pub fn wait_for_leaf_exit(&mut self) {
        self.lineage.wait_for_leaf_exit();
        self.inventory.replace_repeating(Vec::new());
    }

    pub fn reap_root(&mut self) {
        self.lineage.reap_root();
    }
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TERMINAL_ROWS,
        cols: TERMINAL_COLUMNS,
        pixel_width: TERMINAL_PIXEL_SIZE,
        pixel_height: TERMINAL_PIXEL_SIZE,
    }
}

fn leaf_entry_with_parent_unique_id(
    identity: super::ProcessIdentity,
    parent_pid: u32,
    session_id: u32,
    parent_unique_id: u64,
) -> InventoryEntry {
    let metadata = super::macos_native::kernel_process_metadata(identity.pid).unwrap();
    InventoryEntry {
        pid: identity.pid,
        parent_pid,
        process_group: identity.pid,
        session_id,
        start_time: identity.start_time,
        unique_id: metadata.unique_id,
        parent_unique_id,
    }
}
