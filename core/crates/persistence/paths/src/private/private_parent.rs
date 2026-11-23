use anyhow::Result;
use std::path::Path;

pub fn validate_home(home: &Path) -> Result<()> {
    super::validation::validate_home(home)
}

pub fn ensure_private_parent(path: &Path) -> Result<()> {
    secure_private_parent(path).map(|_| ())
}

#[cfg(unix)]
pub fn secure_private_parent(path: &Path) -> Result<super::directory::PrivateDirectory> {
    super::validation::validate_absolute(path)?;
    let parent = parent(path)?;
    super::directory::PrivateDirectory::ensure(parent)
}

#[cfg(not(unix))]
pub fn secure_private_parent(path: &Path) -> Result<()> {
    super::validation::validate_absolute(path)?;
    create_private_dir(parent(path)?)
}

fn parent(path: &Path) -> Result<&Path> {
    path.parent()
        .ok_or_else(|| anyhow::anyhow!("durable path has no parent: {}", path.display()))
}

#[cfg(unix)]
pub(super) fn create_private_dir(path: &Path) -> Result<()> {
    super::directory::PrivateDirectory::ensure(path).map(|_| ())
}

#[cfg(not(unix))]
pub(super) fn create_private_dir(path: &Path) -> Result<()> {
    anyhow::bail!(
        "secure durable directories require Unix descriptor APIs: {}",
        path.display()
    )
}
