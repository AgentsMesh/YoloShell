use super::super::schema::OptionalParticipant;
use super::{rollover_fixture::RolloverFixture, validate};
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyDurableRecord, CustodyHolderIdentity, CustodyLeaseId,
    CustodyOffer, CustodyRequest, CustodyRuntimeIdentity,
};
use crate::sha256_digest::Sha256Digest;
use uuid::Uuid;
const FOREIGN_CUSTODY_DIGEST_BYTE: u8 = 17;
const HOLDER_PID: u32 = 401;
const HOLDER_START: u64 = 402;
const READER_HANDLE: u64 = 0x71;
const RECOVERY_DIGEST_BYTE: u8 = 0x7c;
const WRITER_HANDLE: u64 = 0x72;
#[test]
fn nested_rollover_rewrites_only_boot_and_reconstructs_exact_source() {
    let fixture = RolloverFixture::new();
    let target = checkpoint(&fixture.intent);
    let target_rollover = rollover(target);
    assert_eq!(
        target_rollover.source_sha256,
        crate::pty_custody::durable_record_digest(&fixture.source_custody).unwrap()
    );
    let OptionalParticipant::Replace(custody) = &fixture.intent.pty_custody else {
        unreachable!();
    };
    assert_eq!(custody.source_version, target_rollover.source_version);
    assert_eq!(
        custody.source_object_sha256,
        Some(target_rollover.source_sha256)
    );

    let source = checkpoint_source(&fixture.intent, target);
    assert_eq!(rollover(&source), target_rollover);
    assert!(validate(&fixture.intent).is_ok());
}

#[test]
fn recomputed_checkpoint_digests_cannot_cross_top_level_custody() {
    let fixture = RolloverFixture::new();
    let mut intent = fixture.intent;
    rollover_mut(&mut intent).source_sha256 = Sha256Digest::repeated(FOREIGN_CUSTODY_DIGEST_BYTE);
    rehash_checkpoint(&mut intent);
    assert!(validate(&intent).is_err());
}

#[test]
fn exact_runtime_target_pending_is_a_valid_installed_state() {
    let fixture = RolloverFixture::new();
    let mut intent = fixture.intent;
    let pending = CustodyDurableRecord::pending(CustodyRuntimeIdentity::from_record(
        &intent.runtime.target_payload,
    ));
    replace_runtime_custody(&mut intent, pending);
    assert!(validate(&intent).is_ok());
}

#[test]
fn armed_runtime_target_is_not_an_installed_pending_state() {
    let fixture = RolloverFixture::new();
    let mut intent = fixture.intent;
    let armed = armed_runtime_custody(&intent);
    replace_runtime_custody(&mut intent, armed);
    assert!(validate(&intent).is_err());
}

#[test]
fn foreign_target_is_rejected_with_self_consistent_digests() {
    let fixture = RolloverFixture::new();
    let mut intent = fixture.intent;
    let mut foreign = crate::pty_custody::rewrite_boot_only(
        &fixture.source_custody,
        &intent.source_boot_id,
        &intent.target_boot_id,
    )
    .unwrap();
    foreign.runtime.worker_id = Uuid::new_v4();
    replace_boot_only_custody(&mut intent, foreign);
    assert!(validate(&intent).is_err());
}

fn checkpoint(
    intent: &super::super::RebootMigrationIntent,
) -> &crate::worker::StartupCheckpointEvidence {
    let OptionalParticipant::Replace(participant) = &intent.startup_checkpoint else {
        unreachable!();
    };
    &participant.target_payload
}

fn rollover(
    checkpoint: &crate::worker::StartupCheckpointEvidence,
) -> &crate::worker::StartupPtyCustodyRollover {
    checkpoint.pending_pty_custody_rollover.as_ref().unwrap()
}

fn rollover_mut(
    intent: &mut super::super::RebootMigrationIntent,
) -> &mut crate::worker::StartupPtyCustodyRollover {
    let OptionalParticipant::Replace(participant) = &mut intent.startup_checkpoint else {
        unreachable!();
    };
    participant
        .target_payload
        .pending_pty_custody_rollover
        .as_mut()
        .unwrap()
}

fn checkpoint_source(
    intent: &super::super::RebootMigrationIntent,
    target: &crate::worker::StartupCheckpointEvidence,
) -> crate::worker::StartupCheckpointEvidence {
    crate::worker::source_before_reboot_migration(
        target,
        intent.terminal_id,
        intent.runtime_epoch,
        intent.worker_id,
        intent.controller_generation,
        &intent.runtime.target_payload,
        &intent.source_boot_id,
        &intent.target_boot_id,
    )
    .unwrap()
}

fn rehash_checkpoint(intent: &mut super::super::RebootMigrationIntent) {
    let target = checkpoint(intent).clone();
    let source = checkpoint_source(intent, &target);
    let OptionalParticipant::Replace(participant) = &mut intent.startup_checkpoint else {
        unreachable!();
    };
    participant.target_sha256 = Sha256Digest::capture(&target).unwrap();
    participant.source_sha256 = Sha256Digest::capture(&source).unwrap();
}

fn replace_runtime_custody(
    intent: &mut super::super::RebootMigrationIntent,
    target: CustodyDurableRecord,
) {
    let source = crate::pty_custody::source_before_reboot_migration(
        &target,
        &intent.runtime.target_payload,
        &intent.source_boot_id,
        &intent.target_boot_id,
    )
    .unwrap();
    replace_custody(intent, target, source);
}

fn replace_boot_only_custody(
    intent: &mut super::super::RebootMigrationIntent,
    target: CustodyDurableRecord,
) {
    let source = crate::pty_custody::rewrite_boot_only(
        &target,
        &intent.target_boot_id,
        &intent.source_boot_id,
    )
    .unwrap();
    replace_custody(intent, target, source);
}

fn replace_custody(
    intent: &mut super::super::RebootMigrationIntent,
    target: CustodyDurableRecord,
    source: CustodyDurableRecord,
) {
    let OptionalParticipant::Replace(participant) = &mut intent.pty_custody else {
        unreachable!();
    };
    participant.target_sha256 = Sha256Digest::capture(&target).unwrap();
    participant.source_sha256 = Sha256Digest::capture(&source).unwrap();
    participant.target_payload = target;
}

fn armed_runtime_custody(intent: &super::super::RebootMigrationIntent) -> CustodyDurableRecord {
    let pending = CustodyDurableRecord::pending(CustodyRuntimeIdentity::from_record(
        &intent.runtime.target_payload,
    ));
    let request = CustodyRequest::new(
        CustodyLeaseId::new(),
        pending.runtime.clone(),
        CustodyHolderIdentity {
            pid: HOLDER_PID,
            start_time: HOLDER_START,
        },
        intent.runtime.target_payload.auth_token.clone(),
        intent.controller_generation,
    );
    let offer = CustodyOffer::new_with_profile(
        &request,
        CustodyDescriptorSet::macos_process_capability(READER_HANDLE, WRITER_HANDLE),
        crate::pty_custody::CustodyProfile::ActiveRunning,
        Sha256Digest::repeated(RECOVERY_DIGEST_BYTE),
    )
    .unwrap();
    crate::durable_reboot::pty_custody_fixture::arm(&pending, &request, &offer).unwrap()
}
