use super::*;
use crate::process::{MacosProcessCapabilityChildWriter, MacosProcessCapabilityIssuer};
use portable_pty::{PtySize, native_pty_system};

const TEST_CAPABILITY: &str = "macos-secondary-custody-attestation";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const WRONG_LIVE_PEER_ACCEPTED_DETAIL: &str = "wrong live capability peer was accepted";
const TAMPERED_RELEASED_PEER_ACCEPTED_DETAIL: &str =
    "tampered released capability peer was accepted";
const MALFORMED_ATTESTATION_ACCEPTED_DETAIL: &str = "malformed capability attestation was accepted";
const RELEASED_PROFILE_ACCEPTED_DETAIL: &str = "released profile was accepted for active custody";

#[test]
fn released_profile_is_rejected_at_the_process_import_boundary() {
    let fixture = Fixture::new();
    let export = MacosSecondaryCustodyExport::capture(&fixture.proof, &fixture.runtime).unwrap();
    let digest = export.recovery_identity_sha256();
    let descriptor_set = export.descriptor_set();

    let error = MacosSecondaryCustody::from_transferred(
        export.into_parts(),
        &fixture.runtime,
        digest,
        descriptor_set,
        CustodyProfile::ReleasedTerminal,
    )
    .err()
    .expect(RELEASED_PROFILE_ACCEPTED_DETAIL);

    assert!(error.to_string().contains(PROFILE_DETAIL));
}

#[test]
fn live_nonzero_wrong_peer_is_rejected() {
    let fixture = Fixture::new();
    let export = MacosSecondaryCustodyExport::capture(&fixture.proof, &fixture.runtime).unwrap();
    let digest = export.recovery_identity_sha256();
    let (reader_handle, _) = handles(export.descriptor_set());
    let (foreign_reader, _foreign_writer) = staged_capability();
    let descriptor_set = CustodyDescriptorSet::macos_process_capability(
        reader_handle,
        foreign_reader.identity().writer_handle(),
    );

    MacosSecondaryCustody::from_transferred(
        export.into_parts(),
        &fixture.runtime,
        digest,
        descriptor_set,
        CustodyProfile::ActiveRunning,
    )
    .err()
    .expect(WRONG_LIVE_PEER_ACCEPTED_DETAIL);
}

#[test]
fn malformed_attestation_is_rejected_at_the_process_boundary() {
    let fixture = Fixture::new();
    let export = MacosSecondaryCustodyExport::capture(&fixture.proof, &fixture.runtime).unwrap();
    let digest = export.recovery_identity_sha256();
    let descriptor_set = CustodyDescriptorSet::macos_process_capability(
        crate::pty_custody::INVALID_PIPE_HANDLE,
        crate::pty_custody::INVALID_PIPE_HANDLE,
    );

    MacosSecondaryCustody::from_transferred(
        export.into_parts(),
        &fixture.runtime,
        digest,
        descriptor_set,
        CustodyProfile::ActiveRunning,
    )
    .err()
    .expect(MALFORMED_ATTESTATION_ACCEPTED_DETAIL);
}

#[test]
fn released_writer_uses_attested_identity_for_exact_digest() {
    let fixture = Fixture::new();
    let export = MacosSecondaryCustodyExport::capture(&fixture.proof, &fixture.runtime).unwrap();
    let digest = export.recovery_identity_sha256();
    let descriptor_set = export.descriptor_set();
    let parts = export.into_parts();
    drop(fixture.writer);

    let custody = MacosSecondaryCustody::from_transferred(
        parts,
        &fixture.runtime,
        digest,
        descriptor_set,
        CustodyProfile::ActiveRunning,
    )
    .unwrap();

    assert_eq!(custody.recovery_identity_sha256(), digest);
}

#[test]
fn released_writer_with_tampered_attestation_fails_composite_digest() {
    let fixture = Fixture::new();
    let export = MacosSecondaryCustodyExport::capture(&fixture.proof, &fixture.runtime).unwrap();
    let digest = export.recovery_identity_sha256();
    let (reader_handle, _) = handles(export.descriptor_set());
    let (foreign_reader, _foreign_writer) = staged_capability();
    let descriptor_set = CustodyDescriptorSet::macos_process_capability(
        reader_handle,
        foreign_reader.identity().writer_handle(),
    );
    let parts = export.into_parts();
    drop(fixture.writer);

    let error = MacosSecondaryCustody::from_transferred(
        parts,
        &fixture.runtime,
        digest,
        descriptor_set,
        CustodyProfile::ActiveTerminalReadOnly,
    )
    .err()
    .expect(TAMPERED_RELEASED_PEER_ACCEPTED_DETAIL);

    assert!(error.to_string().contains(IDENTITY_DIGEST_MISMATCH_DETAIL));
}

struct Fixture {
    _pair: portable_pty::PtyPair,
    writer: MacosProcessCapabilityChildWriter,
    proof: PtySessionProof,
    runtime: CustodyRuntimeIdentity,
}

impl Fixture {
    fn new() -> Self {
        let pair = native_pty_system().openpty(pty_size()).unwrap();
        let mut proof =
            PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned())
                .unwrap();
        let (reader, writer) = staged_capability();
        proof.install_process_capability(reader).unwrap();
        let runtime = runtime();
        let _alias_release = proof
            .bind_secondary_custody_alias_for_test(&runtime)
            .unwrap();
        Self {
            _pair: pair,
            writer,
            proof,
            runtime,
        }
    }
}

fn staged_capability() -> (
    Arc<MacosProcessCapabilityManagerReader>,
    MacosProcessCapabilityChildWriter,
) {
    MacosProcessCapabilityIssuer::create()
        .unwrap()
        .into_staged()
        .unwrap()
}

fn handles(descriptor_set: CustodyDescriptorSet) -> (u64, u64) {
    match descriptor_set {
        CustodyDescriptorSet::MacosProcessCapability {
            reader_handle,
            writer_handle,
        } => (reader_handle, writer_handle),
        _ => unreachable!(),
    }
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
