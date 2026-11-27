use crate::delivery_commit::validate_namespace;
use crate::delivery_files::ProtectedFile;
use crate::delivery_receipt::DeliveryReceipt;
use crate::delivery_sync::sync_tree;
use crate::delivery_target::DeliveryTarget;
use crate::delivery_tree::verify_tree;
use crate::developer_release_archive::DeveloperReleaseArchive;
use crate::developer_release_command::DeveloperReleaseCommandRunner;
use anyhow::Result;
use yolo_release_contracts::CHECKSUM_NAME;
use yolo_release_contracts::{DELIVERY_OUTPUT_ENTRIES_ERROR, DELIVERY_OUTPUT_FILE_NAMES};
use yolo_release_evidence_model::ReleaseProvenance;
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_gate_system::validate_named_entries;
use yolo_release_verification::verify::VerifiedDeliveryCandidate;

pub fn verify_stage(
    root: &DirectoryHandle,
    archive: &DeveloperReleaseArchive,
    checksum: &ProtectedFile,
    checksum_bytes: &[u8],
    runner: &dyn DeveloperReleaseCommandRunner,
) -> Result<()> {
    verify_artifacts(root, archive, checksum, checksum_bytes)?;
    runner.verify_signature(root, false)?;
    runner.validate_ticket(root, false)?;
    runner.assess(root, false)
}

#[allow(clippy::too_many_arguments)]
pub fn confirm_published(
    target: &DeliveryTarget,
    delivered: &DirectoryHandle,
    candidate: &VerifiedDeliveryCandidate,
    receipt: &DeliveryReceipt,
    runner: &dyn DeveloperReleaseCommandRunner,
    archive: &DeveloperReleaseArchive,
    checksum: &ProtectedFile,
    checksum_bytes: &[u8],
) -> Result<ReleaseProvenance> {
    validate_namespace(target, delivered)?;
    receipt.record().validate_stage(delivered)?;
    verify_stage(delivered, archive, checksum, checksum_bytes, runner)?;
    sync_tree(delivered)?;
    delivered.durability_barrier()?;
    target.parent.durability_barrier()?;
    receipt.validate(target, candidate)?;
    candidate.revalidate()?;
    verify_artifacts(delivered, archive, checksum, checksum_bytes)?;
    validate_namespace(target, delivered)?;
    receipt.record().validate_stage(delivered)?;
    delivered.verify_self()?;
    Ok(candidate.provenance().clone())
}

fn verify_artifacts(
    root: &DirectoryHandle,
    archive: &DeveloperReleaseArchive,
    checksum: &ProtectedFile,
    checksum_bytes: &[u8],
) -> Result<()> {
    validate_named_entries(
        root,
        DELIVERY_OUTPUT_FILE_NAMES,
        DELIVERY_OUTPUT_ENTRIES_ERROR,
    )?;
    verify_tree(root, archive.inventory())?;
    archive.verify_at(root)?;
    checksum.verify_contents(root, CHECKSUM_NAME, checksum_bytes)
}
