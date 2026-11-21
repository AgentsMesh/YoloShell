use super::*;
use crate::process::{MacosProcessCapabilityChildWriter, MacosProcessCapabilityIssuer};
use portable_pty::{PtySize, native_pty_system};
use std::os::fd::AsRawFd;

const TEST_CAPABILITY: &str = "macos-secondary-custody-test";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const CHANGED_START_TIME_INCREMENT: u64 = 1;
const PTY_NAME_CAPACITY: usize = 128;
const PTY_MASTER_DESCRIPTOR_INDEX: usize = 0;
const PROCESS_READER_DESCRIPTOR_INDEX: usize = 1;
const FCNTL_FAILURE: libc::c_int = -1;
const SYSCALL_FAILURE: libc::c_int = -1;
const REVERSED_ROLES_ACCEPTED_DETAIL: &str = "reversed custody descriptor roles were accepted";
const FOREIGN_READER_ACCEPTED_DETAIL: &str = "foreign capability reader was accepted";

struct Fixture {
    _pair: portable_pty::PtyPair,
    _writer: MacosProcessCapabilityChildWriter,
    proof: PtySessionProof,
    runtime: CustodyRuntimeIdentity,
}

#[test]
fn export_and_transferred_custody_share_one_stable_identity_digest() {
    let Fixture {
        _pair: pair,
        _writer,
        proof,
        runtime,
    } = fixture();
    let exported = MacosSecondaryCustodyExport::capture(&proof, &runtime).unwrap();
    let expected_identity = exported.identity().clone();
    let expected_digest = exported.recovery_identity_sha256();
    let descriptor_set = exported.descriptor_set();
    assert_exact_descriptor_order(exported.as_descriptors());
    let parts = exported.into_parts();
    drop((proof, pair));

    let custody = MacosSecondaryCustody::from_transferred(
        parts,
        &runtime,
        expected_digest,
        descriptor_set,
        CustodyProfile::ActiveRunning,
    )
    .unwrap();

    assert_eq!(custody.identity(), &expected_identity);
    assert_eq!(custody.recovery_identity_sha256(), expected_digest);
    assert_exact_descriptor_order(custody.as_descriptors());
}

#[test]
fn identity_digest_is_replay_stable_and_foreign_runtime_is_rejected() {
    let fixture = fixture();
    let first = MacosSecondaryCustodyExport::capture(&fixture.proof, &fixture.runtime).unwrap();
    let replay = MacosSecondaryCustodyExport::capture(&fixture.proof, &fixture.runtime).unwrap();
    let mut changed_runtime = fixture.runtime.clone();
    changed_runtime.child_start_time += CHANGED_START_TIME_INCREMENT;
    let changed = MacosSecondaryCustodyExport::capture(&fixture.proof, &changed_runtime);

    assert_eq!(
        first.recovery_identity_sha256(),
        replay.recovery_identity_sha256()
    );
    assert_ne!(
        first.recovery_identity_sha256(),
        Sha256Digest::capture(first.identity()).unwrap()
    );
    assert!(changed.is_err());
    assert_eq!(
        serde_json::to_vec(first.identity()).unwrap(),
        serde_json::to_vec(replay.identity()).unwrap()
    );
}

#[test]
fn reversed_order_is_rejected_and_closes_both_descriptors() {
    let fixture = fixture();
    let exported = MacosSecondaryCustodyExport::capture(&fixture.proof, &fixture.runtime).unwrap();
    let digest = exported.recovery_identity_sha256();
    let descriptor_set = exported.descriptor_set();
    let parts = exported.into_parts();
    let raw = descriptor_probes([&parts.process_reader, &parts.pty_master]);
    let reversed = MacosSecondaryCustodyDescriptorParts::from_ordered([
        parts.process_reader,
        parts.pty_master,
    ]);

    MacosSecondaryCustody::from_transferred(
        reversed,
        &fixture.runtime,
        digest,
        descriptor_set,
        CustodyProfile::ActiveRunning,
    )
    .err()
    .expect(REVERSED_ROLES_ACCEPTED_DETAIL);

    assert_closed(raw);
}

#[test]
fn foreign_reader_changes_digest_and_closes_the_received_bundle() {
    let fixture = fixture();
    let exported = MacosSecondaryCustodyExport::capture(&fixture.proof, &fixture.runtime).unwrap();
    let digest = exported.recovery_identity_sha256();
    let descriptor_set = exported.descriptor_set();
    let parts = exported.into_parts();
    let (foreign, _foreign_writer) = MacosProcessCapabilityIssuer::create()
        .unwrap()
        .into_staged()
        .unwrap();
    let foreign_reader = foreign.duplicate_for_secondary().unwrap();
    let raw = descriptor_probes([&parts.pty_master, &foreign_reader]);
    drop(parts.process_reader);
    let substituted =
        MacosSecondaryCustodyDescriptorParts::from_ordered([parts.pty_master, foreign_reader]);

    MacosSecondaryCustody::from_transferred(
        substituted,
        &fixture.runtime,
        digest,
        descriptor_set,
        CustodyProfile::ActiveRunning,
    )
    .err()
    .expect(FOREIGN_READER_ACCEPTED_DETAIL);

    assert_closed(raw);
}

fn fixture() -> Fixture {
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
    let runtime = runtime();
    let _alias_release = proof
        .bind_secondary_custody_alias_for_test(&runtime)
        .unwrap();
    Fixture {
        _pair: pair,
        _writer: writer,
        proof,
        runtime,
    }
}

fn runtime() -> CustodyRuntimeIdentity {
    let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
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

include!("macos_secondary_custody_descriptor_test_support.rs");
