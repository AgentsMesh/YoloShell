use super::constants::{INTENT_INVALID, ROLLOVER_SOURCE_PROOF_INVALID, ROLLOVER_TARGET_INVALID};
use super::schema::RebootMigrationIntent;
use crate::agent::claude::generation::{AuthorityProcessIdentity, ClaudeInvocationAuthorities};
use crate::claude_custody::CustodyLedger;
use crate::model::RuntimeRecord;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::{Result, ensure};
use yolo_startup_checkpoint_contract::StartupCheckpointEvidence;

pub(super) fn validate_startup(intent: &RebootMigrationIntent) -> Result<()> {
    super::validation::validate_optional(
        &intent.startup_checkpoint,
        |target: &StartupCheckpointEvidence| {
            yolo_startup_checkpoint_contract::source_before_reboot_migration(
                target,
                intent.terminal_id,
                intent.runtime_epoch,
                intent.worker_id,
                intent.controller_generation,
                &intent.runtime.target_payload,
                &intent.source_boot_id,
                &intent.target_boot_id,
            )
        },
    )
    .map_err(|error| error.context("startup checkpoint transition"))?;
    validate_custody_rollover(intent).map_err(|error| error.context("startup custody rollover"))
}

fn validate_custody_rollover(intent: &RebootMigrationIntent) -> Result<()> {
    let super::schema::OptionalParticipant::Replace(checkpoint) = &intent.startup_checkpoint else {
        return Ok(());
    };
    let Some(rollover) = checkpoint
        .target_payload
        .pending_pty_custody_rollover
        .as_ref()
    else {
        return Ok(());
    };
    let super::schema::OptionalParticipant::Replace(custody) = &intent.pty_custody else {
        anyhow::bail!(INTENT_INVALID);
    };
    let pending = CustodyDurableRecord::pending(
        crate::pty_custody::CustodyRuntimeIdentity::from_record(&intent.runtime.target_payload),
    );
    ensure!(
        custody.source_version == rollover.source_version
            && custody.source_object_sha256 == Some(rollover.source_sha256),
        ROLLOVER_SOURCE_PROOF_INVALID
    );
    let target_is_pending = custody.target_payload == pending;
    let target_reconstructs_source = crate::pty_custody::rewrite_boot_only(
        &custody.target_payload,
        &intent.target_boot_id,
        &intent.source_boot_id,
    )
    .is_ok_and(|source| {
        crate::sha256_digest::Sha256Digest::capture(&source)
            .is_ok_and(|digest| digest == custody.source_sha256)
            && crate::pty_custody::durable_record_digest(&source)
                .is_ok_and(|digest| digest == rollover.source_sha256)
    });
    ensure!(
        target_is_pending || target_reconstructs_source,
        ROLLOVER_TARGET_INVALID
    );
    Ok(())
}

pub(super) fn validate_claude_custody(intent: &RebootMigrationIntent) -> Result<()> {
    super::validation::validate_optional(&intent.claude_custody, |target: &CustodyLedger| {
        crate::claude_custody::source_before_reboot_migration(
            target,
            &intent.runtime.target_payload,
            &intent.source_boot_id,
            &intent.target_boot_id,
        )
    })
}

pub(super) fn validate_claude_generation(intent: &RebootMigrationIntent) -> Result<()> {
    let worker = AuthorityProcessIdentity {
        pid: intent.runtime.target_payload.worker_pid,
        start_time: intent.runtime.target_payload.worker_start_time,
    };
    super::validation::validate_optional(
        &intent.claude_generation,
        |target: &ClaudeInvocationAuthorities| {
            crate::agent::claude::generation::source_before_reboot_migration(
                target,
                intent.terminal_id,
                intent.runtime_epoch,
                worker,
                &intent.source_boot_id,
                &intent.target_boot_id,
            )
        },
    )
}

pub(super) fn validate_pty_custody(intent: &RebootMigrationIntent) -> Result<()> {
    if let (
        super::schema::OptionalParticipant::Replace(checkpoint),
        super::schema::OptionalParticipant::Replace(custody),
    ) = (&intent.startup_checkpoint, &intent.pty_custody)
        && checkpoint
            .target_payload
            .pending_pty_custody_rollover
            .as_ref()
            .is_some_and(|rollover| {
                custody.source_version == rollover.source_version
                    && custody.source_object_sha256 == Some(rollover.source_sha256)
            })
    {
        return super::validation::validate_optional(&intent.pty_custody, |target| {
            crate::pty_custody::rewrite_boot_only(
                target,
                &intent.target_boot_id,
                &intent.source_boot_id,
            )
        });
    }
    super::validation::validate_optional(&intent.pty_custody, |target: &CustodyDurableRecord| {
        crate::pty_custody::source_before_reboot_migration(
            target,
            &intent.runtime.target_payload,
            &intent.source_boot_id,
            &intent.target_boot_id,
        )
    })
}

pub(super) fn validate_runtime(intent: &RebootMigrationIntent) -> Result<()> {
    super::validation::validate_transition(&intent.runtime, |target: &RuntimeRecord| {
        ensure!(
            runtime_matches_intent(target, intent, &intent.target_boot_id),
            INTENT_INVALID
        );
        let mut source = target.clone();
        source.boot_id.clone_from(&intent.source_boot_id);
        ensure!(
            runtime_matches_intent(&source, intent, &intent.source_boot_id),
            INTENT_INVALID
        );
        Ok(source)
    })
}

fn runtime_matches_intent(
    record: &RuntimeRecord,
    intent: &RebootMigrationIntent,
    expected_boot_id: &str,
) -> bool {
    record.schema_version == crate::constants::RUNTIME_RECORD_SCHEMA_VERSION
        && record.terminal_id == intent.terminal_id
        && record.runtime_epoch == intent.runtime_epoch
        && record.worker_id == intent.worker_id
        && record.controller_generation == intent.controller_generation
        && record.worker_pid != u32::default()
        && record.worker_start_time != u64::default()
        && record.child_pid != u32::default()
        && record.child_start_time != u64::default()
        && record.worker_pid != record.child_pid
        && record.started_at_ms > i64::default()
        && record.socket_path.is_absolute()
        && !record.auth_token.is_empty()
        && !record.command.is_empty()
        && record.cwd.is_absolute()
        && record.cols >= crate::model::MIN_TERMINAL_COLUMNS
        && record.rows >= crate::model::MIN_TERMINAL_ROWS
        && record.boot_id == expected_boot_id
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/runtime_identity_matrix_tests.rs"]
mod identity_matrix_tests;
