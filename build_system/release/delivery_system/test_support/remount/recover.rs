use crate::delivery::{DeliveryHooks, DeliveryRequest, deliver_with};
use crate::delivery_fixture::{ExtractionKind, FakeRunner};
use crate::delivery_recovery_support::{copy_tree, receipt_path};
use crate::record::{OUTPUT_NAME, Phase, REJECTED_COPY_NAME, RETAINED_NAME, Record, child};
use anyhow::{Context, Result, ensure};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use yolo_release_contracts::DELIVERY_RECOVERY_CONFLICT_ERROR;
use yolo_release_foundation::digest::bytes_sha256;
use yolo_release_verification::verify::VerifiedDeliveryCandidate;

struct Continue;
impl DeliveryHooks for Continue {}

pub fn run(mount: &Path, manifest: &Path) -> Result<()> {
    let records: Vec<Record> = serde_json::from_slice(&fs::read(manifest)?)?;
    ensure!(!records.is_empty(), "missing recovery fixtures");
    for record in records {
        let parent = child(mount, &record.directory)?;
        let device = fs::metadata(&parent)?.dev();
        ensure!(
            device != record.device_before,
            "device did not change; remount not proven"
        );
        let candidate = child(&parent, &record.candidate_name)?;
        let output = parent.join(OUTPUT_NAME);
        let receipt = receipt_path(&output);
        ensure!(bytes_sha256(&fs::read(&receipt)?) == record.receipt_sha256);
        let original = match record.phase {
            Phase::Prepared => child(&parent, &record.stage_name)?,
            Phase::Committed => output.clone(),
        };
        let retained = parent.join(RETAINED_NAME);
        let rejected = parent.join(REJECTED_COPY_NAME);
        ensure!(
            !retained.exists() && !rejected.exists(),
            "acceptance already attempted"
        );
        fs::rename(&original, &retained)?;
        copy_tree(&retained, &original);
        let request = || DeliveryRequest {
            candidate_directory: &candidate,
            output_directory: &output,
        };
        let rejected_runner = FakeRunner::new(ExtractionKind::Exact, false);
        let error = deliver_with(request(), &rejected_runner, &Continue)
            .err()
            .context("copied object was accepted")?;
        ensure!(
            format!("{error:#}").contains(DELIVERY_RECOVERY_CONFLICT_ERROR),
            "{error:#}"
        );
        ensure!(rejected_runner.extraction_calls() == 0 && rejected_runner.signature_calls() == 0);
        fs::rename(&original, &rejected)?;
        fs::rename(&retained, &original)?;
        let expected = VerifiedDeliveryCandidate::open(&candidate)?
            .provenance()
            .clone();
        let runner = FakeRunner::new(ExtractionKind::Exact, false);
        ensure!(deliver_with(request(), &runner, &Continue)? == expected);
        ensure!(
            runner.extraction_calls() == 0,
            "recovery unexpectedly re-extracted"
        );
        ensure!(bytes_sha256(&fs::read(&receipt)?) == record.receipt_sha256);
        ensure!(output.is_dir() && !child(&parent, &record.stage_name)?.exists());
        println!(
            "{:?}: changed device {} -> {device}; copy rejected; original recovered",
            record.phase, record.device_before
        );
    }
    Ok(())
}
