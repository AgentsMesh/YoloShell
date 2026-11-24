use super::*;

/// Resolve an existing coordination namespace without creating or repairing its directory.
pub fn existing_lock_path(
    scope: CoordinationScope,
    logical_path: &Path,
    discriminator: Option<&str>,
) -> Result<PathBuf> {
    let (root, path) = lock_location(scope, logical_path, discriminator)?;
    crate::paths::directory::PrivateDirectory::open(&root)?;
    Ok(path)
}

pub(super) fn lock_location(
    scope: CoordinationScope,
    logical_path: &Path,
    discriminator: Option<&str>,
) -> Result<(PathBuf, PathBuf)> {
    crate::paths::validation::validate_absolute(logical_path)?;
    let logical_path = canonical::canonical_logical_path(logical_path)?;
    let root = coordination_root();
    let canonical_root = canonical::canonical_logical_path(&root)?;
    anyhow::ensure!(
        !logical_path.starts_with(&canonical_root) && !canonical_root.starts_with(&logical_path),
        MANAGED_OVERLAP_ERROR
    );
    let digest = lock_digest(scope, &logical_path, discriminator);
    let path = root.join(format!("{}-{digest}{LOCK_FILE_EXTENSION}", scope.label()));
    Ok((root, path))
}
