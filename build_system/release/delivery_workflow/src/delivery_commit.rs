use crate::delivery::{DeliveryCheckpoint, DeliveryHooks};
use crate::delivery_archive::DeliveryArchive;
use crate::delivery_files::ProtectedFile;
use crate::delivery_receipt::DeliveryReceipt;
use crate::delivery_sync::sync_tree;
use crate::delivery_target::DeliveryTarget;
use crate::delivery_tree::verify_tree;
use anyhow::Result;
use yolo_release_contracts::*;
use yolo_release_contracts::{CHECKSUM_NAME, NO_FILE_FLAGS, PRIVATE_DIRECTORY_MODE};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_gate_system::{validate_directory, validate_named_entries, validate_parent};
use yolo_release_verification::verify::VerifiedDeliveryCandidate;

pub struct PublishedEvidence<'a> {
    pub archive: &'a DeliveryArchive,
    pub checksum: &'a ProtectedFile,
    pub checksum_bytes: &'a [u8],
    pub receipt: &'a DeliveryReceipt,
}

pub fn confirm(
    target: &DeliveryTarget,
    delivered: &DirectoryHandle,
    candidate: &VerifiedDeliveryCandidate,
    evidence: PublishedEvidence<'_>,
    hooks: &dyn DeliveryHooks,
) -> Result<()> {
    hooks.checkpoint(DeliveryCheckpoint::PublishedDeliverySync)?;
    sync_tree(delivered)?;
    delivered.durability_barrier()?;
    hooks.checkpoint(DeliveryCheckpoint::PublishedParentSync)?;
    target.parent.durability_barrier()?;
    hooks.checkpoint(DeliveryCheckpoint::PublishedNamespaceValidation)?;
    validate_namespace(target, delivered)?;
    hooks.checkpoint(DeliveryCheckpoint::PublishedTreeValidation)?;
    verify_tree(delivered, candidate.inventory())?;
    hooks.checkpoint(DeliveryCheckpoint::PublishedArchiveValidation)?;
    evidence.archive.verify_at(delivered, candidate)?;
    hooks.checkpoint(DeliveryCheckpoint::PublishedChecksumValidation)?;
    evidence
        .checksum
        .verify_contents(delivered, CHECKSUM_NAME, evidence.checksum_bytes)?;
    delivered.verify_self()?;
    hooks.checkpoint(DeliveryCheckpoint::PublishedReceiptValidation)?;
    evidence.receipt.validate(target, candidate)?;
    hooks.checkpoint(DeliveryCheckpoint::PublishedCandidateValidation)?;
    candidate.revalidate()
}

pub fn validate_namespace(target: &DeliveryTarget, delivered: &DirectoryHandle) -> Result<()> {
    validate_parent(&target.parent)?;
    target.parent.verify_child(&target.destination, delivered)?;
    validate_directory(
        delivered,
        DELIVERY_RENAME_ERROR,
        PRIVATE_DIRECTORY_MODE,
        NO_FILE_FLAGS,
    )?;
    validate_named_entries(
        delivered,
        DELIVERY_OUTPUT_FILE_NAMES,
        DELIVERY_OUTPUT_ENTRIES_ERROR,
    )
}
