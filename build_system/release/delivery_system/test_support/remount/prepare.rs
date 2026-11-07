use crate::delivery::{DeliveryCheckpoint, DeliveryHooks, deliver_with};
use crate::delivery_fixture::{DeliveryFixture, ExtractionKind, FakeRunner};
use crate::delivery_receipt::DeliveryReceipt;
use crate::delivery_recovery_support::receipt_path;
use crate::record::{OUTPUT_NAME, Phase, Record, STOP_MESSAGE};
use anyhow::{Context, Result, bail, ensure};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use yolo_release_delivery_foundation::delivery_target::DeliveryTarget;
use yolo_release_foundation::digest::bytes_sha256;

struct StopAt(DeliveryCheckpoint);
impl DeliveryHooks for StopAt {
    fn checkpoint(&self, checkpoint: DeliveryCheckpoint) -> Result<()> {
        if checkpoint == self.0 {
            bail!(STOP_MESSAGE);
        }
        Ok(())
    }
}

pub fn run(mount: &Path, path: &Path) -> Result<()> {
    let mut manifest = OpenOptions::new().write(true).create_new(true).open(path)?;
    let mut records = Vec::new();
    for phase in [Phase::Prepared, Phase::Committed] {
        let fixture = DeliveryFixture::new_in(tempfile::tempdir_in(mount)?);
        let parent = fixture.delivery.parent().context("fixture parent")?;
        let error = deliver_with(
            fixture.request(),
            &FakeRunner::new(ExtractionKind::Exact, false),
            &StopAt(phase.checkpoint()),
        )
        .err()
        .context("fixture must stop after intent")?;
        ensure!(format!("{error:#}").contains(STOP_MESSAGE), "{error:#}");
        let target = DeliveryTarget::prepare(&fixture.delivery)?;
        let receipt = DeliveryReceipt::open(&target)?.context("durable receipt missing")?;
        records.push(Record {
            phase,
            directory: parent.file_name().context("fixture directory")?.into(),
            candidate_name: fixture
                .candidate
                .file_name()
                .context("candidate name")?
                .into(),
            stage_name: receipt.record().stage_name().into(),
            device_before: fs::metadata(parent)?.dev(),
            receipt_sha256: bytes_sha256(&fs::read(receipt_path(&fixture.delivery))?),
        });
        drop((receipt, target));
        // A later process must reopen the original sealed candidate and receipt.
        std::mem::forget(fixture);
    }
    manifest.write_all(&serde_json::to_vec(&records)?)?;
    manifest.sync_all()?;
    println!("prepared {} durable recovery fixtures", records.len());
    Ok(())
}

pub fn reject_unsupported(mount: &Path) -> Result<()> {
    let private = tempfile::tempdir_in(mount)?;
    let destination = private.path().join(OUTPUT_NAME);
    let error = DeliveryTarget::prepare(&destination)
        .err()
        .context("unsupported filesystem was accepted")?;
    ensure!(
        matches!(
            error.downcast_ref::<yolo_durable_identity::Error>(),
            Some(yolo_durable_identity::Error::Unsupported(_))
        ),
        "preflight did not reject unsupported filesystem identity: {error:#}"
    );
    ensure!(
        fs::read_dir(private.path())?.next().is_none(),
        "preflight wrote state"
    );
    println!("unsupported filesystem rejected before staging");
    Ok(())
}
