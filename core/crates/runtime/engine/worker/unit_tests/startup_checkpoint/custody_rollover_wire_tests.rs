use super::constants::{
    MAX_PROCESS_ID, PENDING_PTY_CUSTODY_ROLLOVER_FIELD, PTY_CUSTODY_SOURCE_SHA256_FIELD,
    PTY_CUSTODY_SOURCE_VERSION_FIELD,
};
use super::retirement_test_support::{activated_runtime, prepare, successor_for};
use super::*;

const NONCANONICAL_PADDING: &str = "=";
const DISTINCT_PROCESS_ID_OFFSET: u32 = 1;
const MAX_BOOT_IDENTITY_BYTE: u8 = b'b';

#[test]
fn largest_rollover_checkpoint_fits_the_durable_protocol() {
    let fixture = CheckpointFixture::new();
    let predecessor = activated_runtime(&fixture);
    let custody = crate::pty_custody::CustodyDurableRecord::pending(
        crate::pty_custody::CustodyRuntimeIdentity::from_record(&predecessor),
    );
    let custody_path = fixture.paths.pty_custody(predecessor.terminal_id);
    fixture
        .namespace
        .write_json(&custody_path, &custody)
        .unwrap();
    let (_, _, identity): (
        crate::pty_custody::CustodyDurableRecord,
        crate::paths::identity::FileIdentity,
        crate::paths::identity::DurableFileVersion,
    ) = fixture
        .namespace
        .lease()
        .read_json_with_versions(&custody_path)
        .unwrap();

    prepare(&fixture, &successor_for(&fixture), Some(&predecessor)).unwrap();
    let bytes = std::fs::read(fixture.path()).unwrap();
    assert!(
        bytes.len()
            <= usize::try_from(crate::paths::constants::WORKER_STARTUP_CHECKPOINT_MAX_BYTES)
                .unwrap()
    );
    let mut evidence = fixture.read();
    let proof = evidence.pending_pty_custody_rollover.as_ref().unwrap();
    assert_eq!(proof.source_version, identity);
    assert_eq!(
        proof.source_sha256,
        crate::pty_custody::durable_record_digest(&custody).unwrap()
    );

    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        value[PENDING_PTY_CUSTODY_ROLLOVER_FIELD][PTY_CUSTODY_SOURCE_VERSION_FIELD].is_object()
    );
    assert!(value[PENDING_PTY_CUSTODY_ROLLOVER_FIELD][PTY_CUSTODY_SOURCE_SHA256_FIELD].is_string());

    evidence.controller_generation = u64::MAX;
    evidence.boot_id = std::iter::repeat_n(
        char::from(MAX_BOOT_IDENTITY_BYTE),
        crate::process::boot_identity::MAX_BOOT_IDENTITY_BYTES,
    )
    .collect();
    evidence.worker.pid = MAX_PROCESS_ID;
    evidence.worker.start_time = u64::MAX;
    let child = evidence.child.as_mut().unwrap();
    child.pid = MAX_PROCESS_ID - DISTINCT_PROCESS_ID_OFFSET;
    child.start_time = u64::MAX;
    let max_digest = crate::sha256_digest::Sha256Digest::repeated(u8::MAX);
    evidence.launch_sha256 = max_digest;
    evidence.predecessor_runtime_sha256 = Some(max_digest);
    let successor = evidence.successor.as_mut().unwrap();
    successor.controller_generation = u64::MAX;
    successor.launch_sha256 = max_digest;
    let rollover = evidence.pending_pty_custody_rollover.as_mut().unwrap();
    rollover.source_version = super::durable_version::largest();
    rollover.source_sha256 = max_digest;
    super::validation::validate(&evidence).unwrap();
    let mut oversized_boot = evidence.clone();
    oversized_boot
        .boot_id
        .push(char::from(MAX_BOOT_IDENTITY_BYTE));
    assert!(super::validation::validate(&oversized_boot).is_err());
    let worst_case_bytes = serde_json::to_vec_pretty(&evidence).unwrap();
    let max_bytes =
        usize::try_from(crate::paths::constants::WORKER_STARTUP_CHECKPOINT_MAX_BYTES).unwrap();
    assert!(
        worst_case_bytes.len() <= max_bytes,
        "worst-case startup checkpoint is {} bytes, limit is {max_bytes}",
        worst_case_bytes.len()
    );
}

#[test]
fn noncanonical_rollover_proof_is_rejected() {
    let identity_fixture = CheckpointFixture::new();
    let mut value = checkpoint_with_rollover(&identity_fixture);
    value[PENDING_PTY_CUSTODY_ROLLOVER_FIELD][PTY_CUSTODY_SOURCE_VERSION_FIELD]
        .as_object_mut()
        .unwrap()
        .remove(super::durable_version::SOURCE_OBJECT_FIELD);
    assert!(serde_json::from_value::<StartupCheckpointEvidence>(value).is_err());

    let digest_fixture = CheckpointFixture::new();
    let mut value = checkpoint_with_rollover(&digest_fixture);
    let digest = value[PENDING_PTY_CUSTODY_ROLLOVER_FIELD][PTY_CUSTODY_SOURCE_SHA256_FIELD]
        .as_str()
        .unwrap()
        .to_owned();
    value[PENDING_PTY_CUSTODY_ROLLOVER_FIELD][PTY_CUSTODY_SOURCE_SHA256_FIELD] =
        format!("{digest}{NONCANONICAL_PADDING}").into();
    assert!(serde_json::from_value::<StartupCheckpointEvidence>(value).is_err());
}

fn checkpoint_with_rollover(fixture: &CheckpointFixture) -> serde_json::Value {
    let predecessor = activated_runtime(fixture);
    let custody = crate::pty_custody::CustodyDurableRecord::pending(
        crate::pty_custody::CustodyRuntimeIdentity::from_record(&predecessor),
    );
    fixture
        .namespace
        .write_json(
            &fixture.paths.pty_custody(predecessor.terminal_id),
            &custody,
        )
        .unwrap();
    prepare(fixture, &successor_for(fixture), Some(&predecessor)).unwrap();
    serde_json::from_slice(&std::fs::read(fixture.path()).unwrap()).unwrap()
}
