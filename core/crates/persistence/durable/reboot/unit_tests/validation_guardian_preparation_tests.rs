use super::super::{OptionalParticipant, RebootMigrationIntent};
use super::{test_fixture::Fixture, validate};
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyDurableRecord, CustodyGuardianBinding, CustodyHolderIdentity,
    CustodyLeaseId, CustodyOffer, CustodyRequest, CustodyRuntimeIdentity,
};
use crate::sha256_digest::Sha256Digest;
use uuid::Uuid;

const ADVANCED_RUNTIME_GENERATION: u64 = 2;
const AUTH_TOKEN: &str = "guardian-reboot-intent-token";
const BASE_GENERATION: u64 = 1;
const GUARDIAN_PID: u32 = 503;
const GUARDIAN_START: u64 = 504;
const HOLDER_PID: u32 = 501;
const HOLDER_START: u64 = 502;
const READER_HANDLE: u64 = 0x81;
const RECOVERY_DIGEST_BYTE: u8 = 0x82;
const SUCCESSOR_GENERATION_INCREMENT: u64 = 1;
const WRITER_HANDLE: u64 = 0x83;

#[test]
fn initial_guardian_preparation_survives_typed_intent_round_trip() {
    assert_intent_round_trip(initial_preparation);
}

#[test]
fn successor_guardian_preparation_survives_typed_intent_round_trip() {
    assert_intent_round_trip(successor_preparation);
}

#[test]
fn self_consistent_candidate_behind_runtime_is_rejected() {
    let prepared =
        Fixture::try_with_pty_source_at_generation(ADVANCED_RUNTIME_GENERATION, |runtime| {
            prepare(pending(runtime), BASE_GENERATION)
        });
    assert!(prepared.is_err());
}

#[test]
fn self_consistent_predecessor_behind_runtime_is_rejected() {
    let prepared =
        Fixture::try_with_pty_source_at_generation(ADVANCED_RUNTIME_GENERATION, |runtime| {
            let predecessor = arm(pending(runtime), BASE_GENERATION);
            prepare(predecessor, ADVANCED_RUNTIME_GENERATION)
        });
    assert!(prepared.is_err());
}

fn assert_intent_round_trip(
    build_pty: impl FnOnce(&crate::model::RuntimeRecord) -> CustodyDurableRecord,
) {
    let fixture = Fixture::with_pty_source(build_pty);
    let bytes = serde_json::to_vec(&fixture.intent).unwrap();
    let intent: RebootMigrationIntent = serde_json::from_slice(&bytes).unwrap();
    assert!(validate(&intent).is_ok());
    let OptionalParticipant::Replace(participant) = &intent.pty_custody else {
        unreachable!();
    };
    let source = crate::pty_custody::source_before_reboot_migration(
        &participant.target_payload,
        &intent.runtime.target_payload,
        &intent.source_boot_id,
        &intent.target_boot_id,
    )
    .unwrap();
    assert_eq!(source, fixture.source_pty);
    assert_eq!(
        Sha256Digest::capture(&source).unwrap(),
        participant.source_sha256
    );
}

fn initial_preparation(runtime: &crate::model::RuntimeRecord) -> CustodyDurableRecord {
    prepare(pending(runtime), runtime.controller_generation)
}

fn successor_preparation(runtime: &crate::model::RuntimeRecord) -> CustodyDurableRecord {
    let predecessor = arm(pending(runtime), runtime.controller_generation);
    prepare(
        predecessor,
        runtime.controller_generation + SUCCESSOR_GENERATION_INCREMENT,
    )
}

fn pending(runtime: &crate::model::RuntimeRecord) -> CustodyDurableRecord {
    CustodyDurableRecord::pending(CustodyRuntimeIdentity::from_record(runtime))
}

fn arm(record: CustodyDurableRecord, generation: u64) -> CustodyDurableRecord {
    let request = request(&record, generation);
    let offer = offer(&request);
    crate::durable_reboot::pty_custody_fixture::arm(&record, &request, &offer).unwrap()
}

fn prepare(record: CustodyDurableRecord, generation: u64) -> CustodyDurableRecord {
    let request = request(&record, generation);
    let offer = offer(&request);
    let guardian =
        CustodyGuardianBinding::new(Uuid::new_v4(), guardian(), &request, &offer).unwrap();
    record.prepare_guardian(&request, &offer, guardian).unwrap()
}

fn request(record: &CustodyDurableRecord, generation: u64) -> CustodyRequest {
    CustodyRequest::new(
        CustodyLeaseId::new(),
        record.runtime.clone(),
        holder(),
        AUTH_TOKEN.to_owned(),
        generation,
    )
}

fn offer(request: &CustodyRequest) -> CustodyOffer {
    CustodyOffer::new_with_profile(
        request,
        CustodyDescriptorSet::macos_process_capability(READER_HANDLE, WRITER_HANDLE),
        crate::pty_custody::CustodyProfile::ActiveRunning,
        Sha256Digest::repeated(RECOVERY_DIGEST_BYTE),
    )
    .unwrap()
}

fn holder() -> CustodyHolderIdentity {
    CustodyHolderIdentity {
        pid: HOLDER_PID,
        start_time: HOLDER_START,
    }
}

fn guardian() -> CustodyHolderIdentity {
    CustodyHolderIdentity {
        pid: GUARDIAN_PID,
        start_time: GUARDIAN_START,
    }
}
