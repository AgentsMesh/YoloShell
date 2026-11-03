use super::*;
use crate::process::{MacosProcessCapabilityIssuer, PtySessionProof};
use portable_pty::{PtySize, native_pty_system};

const TEST_CAPABILITY: &str = "secondary-custody-lazy-reopen-seal";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;

#[test]
fn retained_master_preserves_slave_path_for_lazy_containment_seal() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    let slave_path = pair.master.tty_name().unwrap();
    let (reader, writer) = MacosProcessCapabilityIssuer::create()
        .unwrap()
        .into_staged()
        .unwrap();
    proof.install_process_capability(reader).unwrap();
    let runtime = runtime();
    let alias = proof
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
    drop((writer, proof, pair, alias));

    assert!(slave_path.exists());
    let (authority, holders) = custody.namespace_loss_containment_snapshot().unwrap();
    assert!(authority.authorizes(&runtime));
    assert!(holders.is_empty());
    assert!(slave_path.exists());
}

fn runtime() -> CustodyRuntimeIdentity {
    let process = ProcessIdentity::capture(std::process::id()).unwrap();
    CustodyRuntimeIdentity {
        terminal_id: uuid::Uuid::new_v4(),
        runtime_epoch: uuid::Uuid::new_v4(),
        worker_id: uuid::Uuid::new_v4(),
        worker_pid: process.pid,
        worker_start_time: process.start_time,
        child_pid: process.pid,
        child_start_time: process.start_time,
        boot_id: crate::process::boot_id(),
    }
}

fn pty_size() -> PtySize {
    PtySize {
        rows: TEST_ROWS,
        cols: TEST_COLUMNS,
        pixel_width: TEST_PIXEL_SIZE,
        pixel_height: TEST_PIXEL_SIZE,
    }
}
