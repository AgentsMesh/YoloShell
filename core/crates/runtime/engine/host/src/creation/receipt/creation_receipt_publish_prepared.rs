pub(crate) struct ReceiptPublishContext<'a> {
    pub(crate) staging: &'a ReceiptDirectory,
    pub(crate) index: &'a ReceiptDirectory,
    pub(crate) target: &'a std::ffi::CStr,
    pub(crate) target_name: &'a str,
    pub(crate) receipt: &'a CreationReceipt,
    pub(crate) encoded: &'a [u8],
}

pub(crate) fn publish_prepared_receipt<F, G>(
    publish: ReceiptPublishContext<'_>,
    staged: &mut ReceiptStagingFile,
    hook: &mut F,
    commit_fence: &mut G,
) -> Result<()>
where
    F: FnMut(ReceiptPublishStage) -> Result<()>,
    G: FnMut() -> Result<()>,
{
    let ReceiptPublishContext {
        staging,
        index,
        target,
        target_name,
        receipt,
        encoded,
    } = publish;
    write_receipt_staging(staging, staged, encoded)?;
    hook(ReceiptPublishStage::BeforePublish)?;
    let staged_path = receipt_entry_path(staging, &staged.name);
    verify_receipt_file(
        staged.identity,
        &staged.file,
        &staged_path,
        CREATION_RECEIPT_FILE_MODE,
    )?;
    verify_receipt_named(
        staging,
        &staged.name,
        staged.identity,
        CREATION_RECEIPT_FILE_MODE,
    )?;
    index.verify_path(CREATION_RECEIPT_INDEX_LABEL)?;
    let installed = move_receipt_exclusive_with_fence(
        staging.file.as_raw_fd(),
        &staged.name,
        index.file.as_raw_fd(),
        target,
        &mut *commit_fence,
    )?;
    if !installed {
        let existing = read_receipt_at(index, target_name)?
            .context("conflicting creation receipt disappeared")?;
        anyhow::ensure!(
            existing == *receipt,
            "creation receipt index identity conflict"
        );
        return Ok(());
    }
    if let Err(error) =
        verify_published_receipt(index, target, target_name, receipt, staged.identity)
    {
        return rollback_receipt_publish(staging, index, target, staged, error);
    }
    hook(ReceiptPublishStage::Published)?;
    staging.file.sync_all()?;
    index.file.sync_all()?;
    staging.verify_path("creation receipt staging")?;
    index.verify_path(CREATION_RECEIPT_INDEX_LABEL)?;
    anyhow::ensure!(
        receipt_name_is_missing(staging, &staged.name)?,
        "creation receipt staging name reappeared"
    );
    verify_published_receipt(index, target, target_name, receipt, staged.identity)
}
