use crate::delivery_archive::DeliveryArchive;
pub use crate::delivery_checkpoint::DeliveryCheckpoint;
use crate::delivery_command::{DeliveryCommandRunner, SystemDeliveryCommandRunner};
use crate::delivery_commit::{PublishedEvidence, confirm};
use crate::delivery_files::ProtectedFile;
use crate::delivery_receipt::{DeliveryReceipt, StagedDeliveryReceipt};
use crate::delivery_receipt_record::DeliveryReceiptRecord;
use crate::delivery_recovery::recover;
use crate::delivery_stage::DeliveryStage;
use crate::delivery_sync::sync_tree;
use crate::delivery_target::DeliveryTarget;
use crate::delivery_tree::verify_tree;
use anyhow::{Context, Result, bail};
use std::io::Write;
use std::path::Path;
use yolo_release_contracts::CHECKSUM_NAME;
use yolo_release_contracts::*;
use yolo_release_evidence_model::ReleaseProvenance;
use yolo_release_gate_system::validate_named_entries;
use yolo_release_verification::validation::checksum_contents;
use yolo_release_verification::verify::VerifiedDeliveryCandidate;

pub struct DeliveryRequest<'a> {
    pub candidate_directory: &'a Path,
    pub output_directory: &'a Path,
}

pub trait DeliveryHooks {
    fn checkpoint(&self, _checkpoint: DeliveryCheckpoint) -> Result<()> {
        Ok(())
    }
}

struct ProductionHooks;

impl DeliveryHooks for ProductionHooks {}

pub fn deliver(request: DeliveryRequest<'_>) -> Result<ReleaseProvenance> {
    deliver_with(request, &SystemDeliveryCommandRunner, &ProductionHooks)
}

pub fn deliver_with(
    request: DeliveryRequest<'_>,
    runner: &dyn DeliveryCommandRunner,
    hooks: &dyn DeliveryHooks,
) -> Result<ReleaseProvenance> {
    let target = DeliveryTarget::prepare(request.output_directory)?;
    let _delivery_lock = target.parent.lock_exclusive()?;
    let candidate = VerifiedDeliveryCandidate::open(request.candidate_directory)?;
    hooks.checkpoint(DeliveryCheckpoint::CandidateVerified)?;
    if let Some(receipt) =
        DeliveryReceipt::open(&target).context(DELIVERY_RECOVERY_CONFLICT_ERROR)?
    {
        return recover(&target, &candidate, &receipt, runner, hooks);
    }
    if target
        .existing()
        .context(DELIVERY_EXISTING_OUTPUT_ERROR)?
        .is_some()
    {
        bail!(DELIVERY_EXISTING_OUTPUT_ERROR);
    }
    target.revalidate_missing()?;

    let mut stage = DeliveryStage::create(&target.parent)?;
    let archive = DeliveryArchive::create(stage.handle(), &candidate)?;
    hooks.checkpoint(DeliveryCheckpoint::ArchiveCopied)?;
    let checksum_bytes = checksum_contents(&candidate.provenance().artifact.sha256).into_bytes();
    let checksum = ProtectedFile::create(stage.handle(), CHECKSUM_NAME, |file| {
        file.write_all(&checksum_bytes).map_err(Into::into)
    })?;
    let extraction = stage.create_extraction()?;
    runner.extract(stage.handle())?;
    hooks.checkpoint(DeliveryCheckpoint::Extracted)?;
    validate_named_entries(&extraction, &[APP_NAME], DELIVERY_ROOT_ENTRIES_ERROR)?;
    verify_tree(&extraction, candidate.inventory())?;
    hooks.checkpoint(DeliveryCheckpoint::TreeVerified)?;
    runner.verify_signature(&extraction)?;
    hooks.checkpoint(DeliveryCheckpoint::SignatureVerified)?;
    stage.install_app(&extraction)?;
    verify_stage(
        &candidate,
        stage.handle(),
        &archive,
        &checksum,
        &checksum_bytes,
    )?;
    hooks.checkpoint(DeliveryCheckpoint::BeforePublish)?;
    verify_stage(
        &candidate,
        stage.handle(),
        &archive,
        &checksum,
        &checksum_bytes,
    )?;
    candidate.revalidate()?;
    sync_tree(stage.handle())?;
    stage.handle().durability_barrier()?;
    target.revalidate_missing()?;
    let record =
        DeliveryReceiptRecord::capture(&target, stage.name()?, stage.handle(), &candidate)?;
    let receipt = StagedDeliveryReceipt::create(&target, record)?
        .publish(&mut stage)
        .context(DELIVERY_PREPARED_UNCONFIRMED_ERROR)?;
    receipt
        .verify(&target)
        .context(DELIVERY_PREPARED_UNCONFIRMED_ERROR)?;
    target
        .parent
        .durability_barrier()
        .context(DELIVERY_PREPARED_UNCONFIRMED_ERROR)?;
    target
        .revalidate_missing()
        .context(DELIVERY_PREPARED_UNCONFIRMED_ERROR)?;
    hooks
        .checkpoint(DeliveryCheckpoint::PreparedBeforeRename)
        .context(DELIVERY_PREPARED_UNCONFIRMED_ERROR)?;
    stage
        .publish(&target.destination)
        .context(DELIVERY_PREPARED_UNCONFIRMED_ERROR)?;
    confirm_fresh(
        &target,
        &candidate,
        &receipt,
        runner,
        hooks,
        &archive,
        &checksum,
        &checksum_bytes,
    )
    .context(DELIVERY_COMMITTED_UNCONFIRMED_ERROR)
}

pub fn verify_stage(
    candidate: &VerifiedDeliveryCandidate,
    stage: &yolo_release_filesystem::DirectoryHandle,
    archive: &DeliveryArchive,
    checksum: &ProtectedFile,
    checksum_bytes: &[u8],
) -> Result<()> {
    validate_named_entries(
        stage,
        DELIVERY_OUTPUT_FILE_NAMES,
        DELIVERY_OUTPUT_ENTRIES_ERROR,
    )?;
    verify_tree(stage, candidate.inventory())?;
    archive.verify_at(stage, candidate)?;
    checksum.verify_contents(stage, CHECKSUM_NAME, checksum_bytes)
}

#[allow(clippy::too_many_arguments)]
fn confirm_fresh(
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
    confirm(
        target,
        &delivered,
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
