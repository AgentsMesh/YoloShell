use crate::delivery_archive::DeliveryArchive;
use crate::delivery_files::ProtectedFile;
use crate::delivery_receipt::{DeliveryReceipt, StagedDeliveryReceipt};
use crate::delivery_receipt_record::DeliveryReceiptRecord;
use crate::delivery_stage::DeliveryStage;
use crate::delivery_target::DeliveryTarget;
use crate::delivery_tree::verify_tree;
use crate::developer_release::{
    DeveloperReleaseCheckpoint, DeveloperReleaseHooks, DeveloperReleaseRequest,
};
#[cfg(test)]
use crate::developer_release_acceptance_policy::TestAcceptanceVerifier;
use crate::developer_release_acceptance_policy::{
    AcceptanceVerifier, PinnedAcceptanceVerifier, load_for_delivery,
};
use crate::developer_release_archive::DeveloperReleaseArchive;
use crate::developer_release_command::{DeveloperReleaseCommandRunner, validate_profile};
use crate::developer_release_constants::*;
use crate::developer_release_executables::ExecutableBaseline;
use crate::developer_release_files::{remove, remove_named_archive};
use crate::developer_release_inventory::{validate_signed_transition, validate_transition};
use crate::developer_release_recovery::recover;
use crate::developer_release_verify::{confirm_published, verify_stage};
use anyhow::{Context, Result, bail};
use std::io::Write;
use yolo_release_contracts::*;
use yolo_release_evidence_model::ReleaseProvenance;
use yolo_release_gate_system::validate_named_entries;
use yolo_release_verification::{validation::checksum_contents, verify::VerifiedDeliveryCandidate};

pub fn run(
    request: DeveloperReleaseRequest<'_>,
    runner: &dyn DeveloperReleaseCommandRunner,
    hooks: &dyn DeveloperReleaseHooks,
    acceptance_now: u64,
) -> Result<ReleaseProvenance> {
    run_with_acceptance_verifier(
        request,
        runner,
        hooks,
        Some(acceptance_now),
        &PinnedAcceptanceVerifier,
    )
}

#[cfg(test)]
pub fn run_without_acceptance(
    request: DeveloperReleaseRequest<'_>,
    runner: &dyn DeveloperReleaseCommandRunner,
    hooks: &dyn DeveloperReleaseHooks,
) -> Result<ReleaseProvenance> {
    run_with_acceptance_verifier(request, runner, hooks, None, &PinnedAcceptanceVerifier)
}

#[cfg(test)]
pub fn run_with_test_acceptance_key(
    request: DeveloperReleaseRequest<'_>,
    runner: &dyn DeveloperReleaseCommandRunner,
    hooks: &dyn DeveloperReleaseHooks,
    acceptance_now: u64,
    public_key: &[u8],
) -> Result<ReleaseProvenance> {
    let verifier = TestAcceptanceVerifier::new(public_key);
    run_with_acceptance_verifier(request, runner, hooks, Some(acceptance_now), &verifier)
}

fn run_with_acceptance_verifier(
    request: DeveloperReleaseRequest<'_>,
    runner: &dyn DeveloperReleaseCommandRunner,
    hooks: &dyn DeveloperReleaseHooks,
    acceptance_now: Option<u64>,
    acceptance_verifier: &dyn AcceptanceVerifier,
) -> Result<ReleaseProvenance> {
    validate_profile(request.notary_profile)?;
    let target = DeliveryTarget::prepare(request.output_directory)?;
    let _lock = target.parent.lock_exclusive()?;
    let candidate = VerifiedDeliveryCandidate::open(request.candidate_directory)?;
    let existing_receipt = DeliveryReceipt::open(&target)?;
    let acceptance = load_for_delivery(
        acceptance_verifier,
        &request,
        &candidate,
        existing_receipt.as_ref(),
        acceptance_now,
    )?;
    if let Some(receipt) = existing_receipt {
        return recover(&target, &candidate, &receipt, runner, acceptance.as_ref());
    }
    if target.existing()?.is_some() {
        bail!(DELIVERY_EXISTING_OUTPUT_ERROR);
    }
    target.revalidate_missing()?;
    let mut stage = DeliveryStage::create(&target.parent)?;
    let candidate_archive = DeliveryArchive::create(stage.handle(), &candidate)?;
    let extraction = stage.create_extraction()?;
    runner.extract(stage.handle(), candidate_archive.retained_file()?)?;
    candidate_archive.verify_at(stage.handle(), &candidate)?;
    validate_named_entries(&extraction, &[APP_NAME], DELIVERY_ROOT_ENTRIES_ERROR)?;
    verify_tree(&extraction, candidate.inventory())?;
    candidate.revalidate()?;
    let baseline = ExecutableBaseline::capture(&extraction, candidate.inventory(), runner)?;
    candidate_archive.verify_at(stage.handle(), &candidate)?;
    crate::developer_release_acceptance::revalidate(acceptance.as_ref(), &candidate)?;
    runner.sign(stage.handle())?;
    baseline.verify(&extraction, runner)?;
    runner.verify_signature(stage.handle(), true)?;
    runner.create_archive(
        stage.handle(),
        &format!("{EXTRACTION_DIRECTORY_NAME}/{APP_NAME}"),
        NOTARY_SUBMISSION_NAME,
    )?;
    let submission = ProtectedFile::seal_existing(stage.handle(), NOTARY_SUBMISSION_NAME)?;
    let submission_inventory = yolo_release_archive_audit::audit_opened_archive(submission.file())?;
    validate_signed_transition(candidate.inventory(), &submission_inventory)?;
    verify_tree(&extraction, &submission_inventory)?;
    candidate_archive.verify_at(stage.handle(), &candidate)?;
    crate::developer_release_acceptance::revalidate(acceptance.as_ref(), &candidate)?;
    let notarization_id = runner.notarize(
        stage.handle(),
        request.notary_profile,
        std::sync::Arc::new(submission.file().try_clone()?),
    )?;
    submission.verify_at(stage.handle(), NOTARY_SUBMISSION_NAME)?;
    remove(stage.handle(), NOTARY_SUBMISSION_NAME, &submission)?;
    runner.staple(stage.handle(), true)?;
    runner.validate_ticket(stage.handle(), true)?;
    runner.verify_signature(stage.handle(), true)?;
    runner.assess(stage.handle(), true)?;
    stage.install_app(&extraction)?;
    drop(candidate_archive);
    remove_named_archive(stage.handle())?;
    let archive = DeveloperReleaseArchive::create(stage.handle(), runner)?;
    validate_transition(candidate.inventory(), archive.inventory())?;
    let checksum_bytes = checksum_contents(archive.digest()).into_bytes();
    let checksum = ProtectedFile::create(stage.handle(), CHECKSUM_NAME, |file| {
        file.write_all(&checksum_bytes).map_err(Into::into)
    })?;
    verify_stage(stage.handle(), &archive, &checksum, &checksum_bytes, runner)?;
    crate::developer_release_acceptance::revalidate(acceptance.as_ref(), &candidate)?;
    crate::delivery_sync::sync_tree(stage.handle())?;
    stage.handle().durability_barrier()?;
    target.revalidate_missing()?;
    let binding = crate::developer_release_binding::DeveloperReleaseBinding::capture(
        archive.digest(),
        archive.size(),
        notarization_id,
        acceptance
            .as_ref()
            .map(yolo_release_acceptance_receipt::AcceptanceReceipt::canonical_digest)
            .transpose()?,
    )?;
    let record = DeliveryReceiptRecord::capture_developer_release(
        &target,
        stage.name()?,
        stage.handle(),
        &candidate,
        binding,
    )?;
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
        .checkpoint(DeveloperReleaseCheckpoint::PreparedBeforeRename)
        .context(DELIVERY_PREPARED_UNCONFIRMED_ERROR)?;
    stage
        .publish(&target.destination)
        .context(DELIVERY_PREPARED_UNCONFIRMED_ERROR)?;
    let delivered = target
        .open()
        .context(DELIVERY_COMMITTED_UNCONFIRMED_ERROR)?;
    receipt
        .record()
        .validate_stage(&delivered)
        .context(DELIVERY_COMMITTED_UNCONFIRMED_ERROR)?;
    let provenance = confirm_published(
        &target,
        &delivered,
        &candidate,
        &receipt,
        runner,
        &archive,
        &checksum,
        &checksum_bytes,
    )
    .context(DELIVERY_COMMITTED_UNCONFIRMED_ERROR)?;
    crate::developer_release_acceptance::revalidate(acceptance.as_ref(), &candidate)
        .context(DELIVERY_COMMITTED_UNCONFIRMED_ERROR)?;
    Ok(provenance)
}
