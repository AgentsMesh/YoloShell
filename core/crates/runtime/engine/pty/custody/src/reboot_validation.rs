use super::{
    CustodyDurableLease, CustodyDurableRecord, CustodyDurableState, CustodyGuardianBinding,
    CustodyRuntimeIdentity,
};
use crate::model::RuntimeRecord;
use crate::sha256_digest::Sha256Digest;
use anyhow::{Result, ensure};

const RECORD_INVALID: &str = "PTY custody reboot record is invalid";
const RECORD_BOOT_INVALID: &str = "PTY custody reboot record boot identity is invalid";
const RECORD_TARGET_BOOT_INVALID: &str =
    "PTY custody reboot record target boot identity is invalid";
const RUNTIME_IDENTITY_INVALID: &str = "PTY custody reboot runtime identity is invalid";
const CUSTODY_DIGEST_DOMAIN: &str = "yolo.predecessor-pty-custody.v1";

pub fn durable_record_digest(record: &CustodyDurableRecord) -> Result<Sha256Digest> {
    Sha256Digest::capture(&(CUSTODY_DIGEST_DOMAIN, record))
}

pub fn rewrite_boot_for_target(
    source: &CustodyDurableRecord,
    current_boot_id: &str,
    target_boot_id: &str,
) -> Result<CustodyDurableRecord> {
    use yolo_process_identity::BootIdentityRelation::Same;
    ensure!(source.is_valid(), RECORD_INVALID);
    ensure!(
        yolo_process_identity::boot_identity_relation(&source.runtime.boot_id, current_boot_id)
            == Same
            || yolo_process_identity::boot_identity_relation(
                &source.runtime.boot_id,
                target_boot_id,
            ) == Same,
        RECORD_TARGET_BOOT_INVALID
    );
    ensure!(
        yolo_process_identity::boot_identity_relation(target_boot_id, target_boot_id) == Same,
        RECORD_TARGET_BOOT_INVALID
    );
    rewrite_boot(source, target_boot_id)
}

pub fn rewrite_boot_only(
    source: &CustodyDurableRecord,
    expected_boot_id: &str,
    replacement_boot_id: &str,
) -> Result<CustodyDurableRecord> {
    use yolo_process_identity::BootIdentityRelation::Same;
    ensure!(source.is_valid(), RECORD_INVALID);
    ensure!(
        yolo_process_identity::boot_identity_relation(&source.runtime.boot_id, expected_boot_id)
            == Same
            && yolo_process_identity::boot_identity_relation(
                replacement_boot_id,
                replacement_boot_id,
            ) == Same,
        RECORD_BOOT_INVALID
    );
    rewrite_boot(source, replacement_boot_id)
}

pub fn source_before_reboot_migration(
    target: &CustodyDurableRecord,
    runtime: &RuntimeRecord,
    source_boot_id: &str,
    target_boot_id: &str,
) -> Result<CustodyDurableRecord> {
    ensure!(target.is_valid(), RECORD_INVALID);
    let expected = super::CustodyRuntimeIdentity::from_record(runtime);
    let generation_matches =
        state_matches_runtime_generation(&target.state, runtime.controller_generation);
    ensure!(
        target.runtime.terminal_id == expected.terminal_id
            && target.runtime.runtime_epoch == expected.runtime_epoch
            && target.runtime.worker_id == expected.worker_id
            && target.runtime.worker_pid == expected.worker_pid
            && target.runtime.worker_start_time == expected.worker_start_time
            && target.runtime.child_pid == expected.child_pid
            && target.runtime.child_start_time == expected.child_start_time
            && yolo_process_identity::boot_identity_relation(
                &target.runtime.boot_id,
                target_boot_id,
            ) == yolo_process_identity::BootIdentityRelation::Same
            && generation_matches,
        RUNTIME_IDENTITY_INVALID
    );
    rewrite_boot_only(target, target_boot_id, source_boot_id)
}

fn state_matches_runtime_generation(state: &CustodyDurableState, minimum: u64) -> bool {
    match state {
        CustodyDurableState::Pending {} => true,
        CustodyDurableState::PreparedGuardian {
            predecessor,
            candidate,
        } => {
            candidate.controller_generation >= minimum
                && state_matches_runtime_generation(predecessor, minimum)
        }
        CustodyDurableState::Armed {
            controller_generation,
            ..
        }
        | CustodyDurableState::Retired {
            controller_generation,
            ..
        } => *controller_generation >= minimum,
    }
}

fn rewrite_boot(
    source: &CustodyDurableRecord,
    target_boot_id: &str,
) -> Result<CustodyDurableRecord> {
    let source_runtime = source.runtime.clone();
    let mut target_runtime = source_runtime.clone();
    target_runtime.boot_id = target_boot_id.to_owned();
    let mut state = source.state.clone();
    rewrite_state(&mut state, &source_runtime, &target_runtime)?;
    let replacement = CustodyDurableRecord {
        schema_version: source.schema_version,
        runtime: target_runtime,
        state,
    };
    ensure!(replacement.is_valid(), RECORD_INVALID);
    Ok(replacement)
}

fn rewrite_state(
    state: &mut CustodyDurableState,
    source: &CustodyRuntimeIdentity,
    target: &CustodyRuntimeIdentity,
) -> Result<()> {
    match state {
        CustodyDurableState::Pending {} => {}
        CustodyDurableState::PreparedGuardian {
            predecessor,
            candidate,
        } => {
            rewrite_state(predecessor, source, target)?;
            rewrite_lease(candidate, source, target)?;
        }
        CustodyDurableState::Armed { guardian, .. }
        | CustodyDurableState::Retired { guardian, .. } => {
            rewrite_guardian(guardian, source, target)?;
        }
    }
    Ok(())
}

fn rewrite_lease(
    lease: &mut CustodyDurableLease,
    source: &CustodyRuntimeIdentity,
    target: &CustodyRuntimeIdentity,
) -> Result<()> {
    rewrite_guardian(&mut lease.guardian, source, target)
}

fn rewrite_guardian(
    guardian: &mut Option<CustodyGuardianBinding>,
    source: &CustodyRuntimeIdentity,
    target: &CustodyRuntimeIdentity,
) -> Result<()> {
    let Some(current) = *guardian else {
        return Ok(());
    };
    *guardian = Some(
        current
            .rewrite_runtime_identity(source, target)
            .ok_or_else(|| anyhow::anyhow!(RECORD_INVALID))?,
    );
    Ok(())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/reboot_validation_tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/reboot_validation_guardian_tests.rs"]
mod guardian_tests;
