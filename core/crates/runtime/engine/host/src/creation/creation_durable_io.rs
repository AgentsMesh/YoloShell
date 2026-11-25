use super::*;

pub(crate) fn verify_private_directory(
    metadata: &std::fs::Metadata,
    path: &Path,
    label: &str,
) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    anyhow::ensure!(
        metadata.file_type().is_dir(),
        "{label} parent is not a directory: {}",
        path.display()
    );
    anyhow::ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        "{label} parent is owned by another user: {}",
        path.display()
    );
    anyhow::ensure!(
        metadata.mode() & NON_OWNER_PERMISSION_MASK == 0,
        "{label} parent permissions are not private: {}",
        path.display()
    );
    Ok(())
}

pub(crate) fn creation_ledger_temporary_prefix() -> String {
    format!(".{CREATION_LEDGER_FILE_NAME}.")
}
