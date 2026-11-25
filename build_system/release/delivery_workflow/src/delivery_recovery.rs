use crate::delivery::{DeliveryCheckpoint, DeliveryHooks, verify_stage};
use crate::delivery_archive::DeliveryArchive;
use crate::delivery_command::DeliveryCommandRunner;
use crate::delivery_commit::{PublishedEvidence, confirm, validate_namespace};
use crate::delivery_files::ProtectedFile;
use crate::delivery_receipt::DeliveryReceipt;
use crate::delivery_target::DeliveryTarget;
use crate::delivery_tree::verify_tree;
use anyhow::{Context, Result, bail, ensure};
use std::ffi::CString;
use yolo_release_contracts::CHECKSUM_NAME;
use yolo_release_contracts::*;
use yolo_release_evidence_model::ReleaseProvenance;
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_gate_system::{ensure_missing, entry_exists, validate_parent};
use yolo_release_verification::validation::checksum_contents;
use yolo_release_verification::verify::VerifiedDeliveryCandidate;

pub fn recover(
    target: &DeliveryTarget,
    candidate: &VerifiedDeliveryCandidate,
    receipt: &DeliveryReceipt,
    runner: &dyn DeliveryCommandRunner,
    hooks: &dyn DeliveryHooks,
) -> Result<ReleaseProvenance> {
    receipt
        .validate(target, candidate)
        .context(DELIVERY_RECOVERY_CONFLICT_ERROR)?;
    receipt
        .record()
        .validate_delivery_kind()
        .context(DELIVERY_RECOVERY_CONFLICT_ERROR)?;
    let destination = target
        .existing()
        .context(DELIVERY_RECOVERY_CONFLICT_ERROR)?;
    let stage_exists = entry_exists(&target.parent, receipt.record().stage_name())?;
    match (destination, stage_exists) {
        (Some(delivered), false) => {
            recover_committed(target, &delivered, candidate, receipt, runner, hooks)
                .context(DELIVERY_COMMITTED_UNCONFIRMED_ERROR)
        }
        (None, true) => recover_prepared(target, candidate, receipt, runner, hooks)
            .context(DELIVERY_PREPARED_UNCONFIRMED_ERROR),
        _ => bail!(DELIVERY_RECOVERY_CONFLICT_ERROR),
    }
}

fn recover_prepared(
    target: &DeliveryTarget,
    candidate: &VerifiedDeliveryCandidate,
    receipt: &DeliveryReceipt,
    runner: &dyn DeliveryCommandRunner,
    hooks: &dyn DeliveryHooks,
) -> Result<ReleaseProvenance> {
    let stage_name = receipt.record().stage_name();
    let stage = DirectoryHandle::open_at(&target.parent, stage_name)?;
    target.parent.verify_child(stage_name, &stage)?;
    receipt.record().validate_stage(&stage)?;
    let archive = DeliveryArchive::open(&stage, candidate)?;
    let checksum_bytes = checksum_contents(&candidate.provenance().artifact.sha256).into_bytes();
    let checksum = ProtectedFile::open(&stage, CHECKSUM_NAME)?;
    verify_stage(candidate, &stage, &archive, &checksum, &checksum_bytes)?;
    runner.verify_signature(&stage)?;
    crate::delivery_sync::sync_tree(&stage)?;
    stage.durability_barrier()?;
    receipt.verify(target)?;
    target.parent.durability_barrier()?;
    ensure_missing(&target.parent, &target.destination)?;
    publish_prepared(target, stage_name, &stage)?;
    open_and_confirm(
        target,
        candidate,
        receipt,
        runner,
        hooks,
        &archive,
        &checksum,
        &checksum_bytes,
    )
}

fn recover_committed(
    target: &DeliveryTarget,
    delivered: &DirectoryHandle,
    candidate: &VerifiedDeliveryCandidate,
    receipt: &DeliveryReceipt,
    runner: &dyn DeliveryCommandRunner,
    hooks: &dyn DeliveryHooks,
) -> Result<ReleaseProvenance> {
    hooks.checkpoint(DeliveryCheckpoint::PublishedDestinationIdentityValidation)?;
    receipt.record().validate_stage(delivered)?;
    validate_namespace(target, delivered)?;
    verify_tree(delivered, candidate.inventory())?;
    let archive = DeliveryArchive::open(delivered, candidate)?;
    let checksum_bytes = checksum_contents(&candidate.provenance().artifact.sha256).into_bytes();
    let checksum = ProtectedFile::open(delivered, CHECKSUM_NAME)?;
    checksum.verify_contents(delivered, CHECKSUM_NAME, &checksum_bytes)?;
    hooks.checkpoint(DeliveryCheckpoint::PublishedSignatureValidation)?;
    runner.verify_signature(delivered)?;
    confirm_delivery(
        target,
        delivered,
        candidate,
        receipt,
        hooks,
        &archive,
        &checksum,
        &checksum_bytes,
    )
}

#[allow(clippy::too_many_arguments)]
fn open_and_confirm(
    target: &DeliveryTarget,
    candidate: &VerifiedDeliveryCandidate,
    receipt: &DeliveryReceipt,
    runner: &dyn DeliveryCommandRunner,
    hooks: &dyn DeliveryHooks,
    archive: &DeliveryArchive,
    checksum: &ProtectedFile,
    checksum_bytes: &[u8],
) -> Result<ReleaseProvenance> {
    hooks.checkpoint(DeliveryCheckpoint::PublishedDestinationOpen)?;
    let delivered = target.open()?;
    hooks.checkpoint(DeliveryCheckpoint::PublishedDestinationIdentityValidation)?;
    receipt.record().validate_stage(&delivered)?;
    hooks.checkpoint(DeliveryCheckpoint::PublishedSignatureValidation)?;
    runner.verify_signature(&delivered)?;
    confirm_delivery(
        target,
        &delivered,
        candidate,
        receipt,
        hooks,
        archive,
        checksum,
        checksum_bytes,
    )
}

#[allow(clippy::too_many_arguments)]
fn confirm_delivery(
    target: &DeliveryTarget,
    delivered: &DirectoryHandle,
    candidate: &VerifiedDeliveryCandidate,
    receipt: &DeliveryReceipt,
    hooks: &dyn DeliveryHooks,
    archive: &DeliveryArchive,
    checksum: &ProtectedFile,
    checksum_bytes: &[u8],
) -> Result<ReleaseProvenance> {
    confirm(
        target,
        delivered,
        candidate,
        PublishedEvidence {
            archive,
            checksum,
            checksum_bytes,
            receipt,
        },
        hooks,
    )?;
    Ok(candidate.provenance().clone())
}

fn publish_prepared(
    target: &DeliveryTarget,
    stage_name: &str,
    stage: &DirectoryHandle,
) -> Result<()> {
    validate_parent(&target.parent)?;
    target.parent.verify_child(stage_name, stage)?;
    let stage_name = CString::new(stage_name)?;
    let destination = CString::new(target.destination.as_str())?;
    ensure!(
        unsafe {
            libc::renameatx_np(
                target.parent.raw_fd(),
                stage_name.as_ptr(),
                target.parent.raw_fd(),
                destination.as_ptr(),
                libc::RENAME_EXCL,
            )
        } == DELIVERY_SYSTEM_CALL_SUCCESS,
        DELIVERY_RENAME_ERROR
    );
    Ok(())
}
