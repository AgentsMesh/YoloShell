use crate::delivery_files::ProtectedFile;
use crate::delivery_receipt::DeliveryReceipt;
use crate::delivery_target::DeliveryTarget;
use crate::developer_release_archive::DeveloperReleaseArchive;
use crate::developer_release_binding::DeveloperReleaseBinding;
use crate::developer_release_command::DeveloperReleaseCommandRunner;
use crate::developer_release_inventory::validate_transition;
use crate::developer_release_verify::{confirm_published, verify_stage};
use anyhow::{Context, Result, bail, ensure};
use std::ffi::CString;
use yolo_release_acceptance_receipt::AcceptanceReceipt;
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
    runner: &dyn DeveloperReleaseCommandRunner,
    acceptance: Option<&AcceptanceReceipt>,
) -> Result<ReleaseProvenance> {
    receipt
        .validate(target, candidate)
        .context(DELIVERY_RECOVERY_CONFLICT_ERROR)?;
    receipt
        .record()
        .developer_release()
        .context(DELIVERY_RECOVERY_CONFLICT_ERROR)?;
    let destination = target
        .existing()
        .context(DELIVERY_RECOVERY_CONFLICT_ERROR)?;
    let stage_exists = entry_exists(&target.parent, receipt.record().stage_name())?;
    match (destination, stage_exists) {
        (Some(delivered), false) => {
            recover_committed(target, &delivered, candidate, receipt, runner, acceptance)
                .context(DELIVERY_COMMITTED_UNCONFIRMED_ERROR)
        }
        (None, true) => recover_prepared(target, candidate, receipt, runner, acceptance)
            .context(DELIVERY_PREPARED_UNCONFIRMED_ERROR),
        _ => bail!(DELIVERY_RECOVERY_CONFLICT_ERROR),
    }
}

fn recover_prepared(
    target: &DeliveryTarget,
    candidate: &VerifiedDeliveryCandidate,
    receipt: &DeliveryReceipt,
    runner: &dyn DeveloperReleaseCommandRunner,
    acceptance: Option<&AcceptanceReceipt>,
) -> Result<ReleaseProvenance> {
    let name = receipt.record().stage_name();
    let stage = DirectoryHandle::open_at(&target.parent, name)?;
    target.parent.verify_child(name, &stage)?;
    receipt.record().validate_stage(&stage)?;
    let binding = receipt.record().developer_release()?;
    let (archive, checksum, checksum_bytes) = open_stage(&stage, candidate, binding, runner)?;
    crate::delivery_sync::sync_tree(&stage)?;
    stage.durability_barrier()?;
    receipt.verify(target)?;
    target.parent.durability_barrier()?;
    ensure_missing(&target.parent, &target.destination)?;
    crate::developer_release_acceptance::revalidate(acceptance, candidate)?;
    publish_prepared(target, name, &stage)?;
    let delivered = target.open()?;
    receipt.record().validate_stage(&delivered)?;
    let provenance = confirm_published(
        target,
        &delivered,
        candidate,
        receipt,
        runner,
        &archive,
        &checksum,
        &checksum_bytes,
    )?;
    crate::developer_release_acceptance::revalidate(acceptance, candidate)?;
    Ok(provenance)
}

fn recover_committed(
    target: &DeliveryTarget,
    delivered: &DirectoryHandle,
    candidate: &VerifiedDeliveryCandidate,
    receipt: &DeliveryReceipt,
    runner: &dyn DeveloperReleaseCommandRunner,
    acceptance: Option<&AcceptanceReceipt>,
) -> Result<ReleaseProvenance> {
    receipt.record().validate_stage(delivered)?;
    let binding = receipt.record().developer_release()?;
    let (archive, checksum, checksum_bytes) = open_stage(delivered, candidate, binding, runner)?;
    crate::developer_release_acceptance::revalidate(acceptance, candidate)?;
    let provenance = confirm_published(
        target,
        delivered,
        candidate,
        receipt,
        runner,
        &archive,
        &checksum,
        &checksum_bytes,
    )?;
    crate::developer_release_acceptance::revalidate(acceptance, candidate)?;
    Ok(provenance)
}

fn open_stage(
    stage: &DirectoryHandle,
    candidate: &VerifiedDeliveryCandidate,
    binding: &DeveloperReleaseBinding,
    runner: &dyn DeveloperReleaseCommandRunner,
) -> Result<(DeveloperReleaseArchive, ProtectedFile, Vec<u8>)> {
    let archive = DeveloperReleaseArchive::open(stage)?;
    binding.validate(archive.digest(), archive.size())?;
    validate_transition(candidate.inventory(), archive.inventory())?;
    let checksum_bytes = checksum_contents(archive.digest()).into_bytes();
    let checksum = ProtectedFile::open(stage, CHECKSUM_NAME)?;
    verify_stage(stage, &archive, &checksum, &checksum_bytes, runner)?;
    Ok((archive, checksum, checksum_bytes))
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
