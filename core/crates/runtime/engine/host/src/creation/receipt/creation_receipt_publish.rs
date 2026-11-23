use super::*;
use std::os::fd::AsRawFd;

pub(crate) fn publish_receipt_at(
    staging: &ReceiptDirectory,
    index: &ReceiptDirectory,
    name: &str,
    receipt: &CreationReceipt,
) -> Result<()> {
    publish_receipt_at_with(staging, index, name, receipt, |_| Ok(()))
}

pub(crate) fn publish_receipt_at_before(
    staging: &ReceiptDirectory,
    index: &ReceiptDirectory,
    name: &str,
    receipt: &CreationReceipt,
    deadline: HostOperationDeadline,
) -> Result<()> {
    publish_receipt_at_with_fence(
        staging,
        index,
        name,
        receipt,
        |_| Ok(()),
        || require_creation_commit_budget(deadline),
    )
}

pub(crate) fn publish_receipt_at_with<F>(
    staging: &ReceiptDirectory,
    index: &ReceiptDirectory,
    name: &str,
    receipt: &CreationReceipt,
    hook: F,
) -> Result<()>
where
    F: FnMut(ReceiptPublishStage) -> Result<()>,
{
    publish_receipt_at_with_fence(staging, index, name, receipt, hook, || Ok(()))
}

pub(crate) fn publish_receipt_at_with_fence<F, G>(
    staging: &ReceiptDirectory,
    index: &ReceiptDirectory,
    name: &str,
    receipt: &CreationReceipt,
    mut hook: F,
    mut commit_fence: G,
) -> Result<()>
where
    F: FnMut(ReceiptPublishStage) -> Result<()>,
    G: FnMut() -> Result<()>,
{
    staging.verify_path("creation receipt staging")?;
    index.verify_path(CREATION_RECEIPT_INDEX_LABEL)?;
    if let Some(existing) = read_receipt_at(index, name)? {
        anyhow::ensure!(
            existing == *receipt,
            "creation receipt index identity conflict"
        );
        return Ok(());
    }
    let encoded = serde_json::to_vec(receipt)?;
    anyhow::ensure!(
        encoded.len() as u64 <= MAX_CREATION_RECEIPT_BYTES,
        "creation receipt exceeds byte limit"
    );
    let target = std::ffi::CString::new(name).context("receipt index name contains a null byte")?;
    let mut staged = acquire_receipt_staging(staging, index, &target)?;
    let publish = ReceiptPublishContext {
        staging,
        index,
        target: &target,
        target_name: name,
        receipt,
        encoded: &encoded,
    };
    let result = publish_prepared_receipt(publish, &mut staged, &mut hook, &mut commit_fence);
    let cleanup =
        retire_receipt_staging(staging, &mut staged).context("clean receipt staging after publish");
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => {
            Err(error.context(format!("receipt staging cleanup also failed: {cleanup:#}")))
        }
    }
}

include!("creation_receipt_publish_prepared.rs");

pub(crate) fn verify_published_receipt(
    index: &ReceiptDirectory,
    target: &std::ffi::CStr,
    target_name: &str,
    receipt: &CreationReceipt,
    expected: ReceiptFileIdentity,
) -> Result<()> {
    verify_receipt_named(index, target, expected, CREATION_RECEIPT_FILE_MODE)?;
    let installed =
        read_receipt_at(index, target_name)?.context("published creation receipt disappeared")?;
    anyhow::ensure!(installed == *receipt, "published creation receipt changed");
    Ok(())
}

pub(crate) fn rollback_receipt_publish(
    staging: &ReceiptDirectory,
    index: &ReceiptDirectory,
    target: &std::ffi::CStr,
    staged: &ReceiptStagingFile,
    cause: anyhow::Error,
) -> Result<()> {
    let restored = move_receipt_exclusive(
        index.file.as_raw_fd(),
        target,
        staging.file.as_raw_fd(),
        &staged.name,
    );
    index.file.sync_all()?;
    staging.file.sync_all()?;
    match restored {
        Ok(true) => Err(cause.context("creation receipt publish was rolled back")),
        Ok(false) => Err(cause.context("creation receipt replacement retained in target index")),
        Err(error) => Err(cause.context(format!(
            "creation receipt rollback failed with both names retained: {error:#}"
        ))),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReceiptPublishStage {
    BeforePublish,
    Published,
}

#[cfg(test)]
pub(crate) fn publish_receipt_at_with_test_hook<F>(
    staging: &ReceiptDirectory,
    index: &ReceiptDirectory,
    name: &str,
    receipt: &CreationReceipt,
    hook: F,
) -> Result<()>
where
    F: FnMut(ReceiptPublishStage) -> Result<()>,
{
    publish_receipt_at_with(staging, index, name, receipt, hook)
}
