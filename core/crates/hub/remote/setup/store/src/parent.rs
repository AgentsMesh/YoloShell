use anyhow::{Result, ensure};
use yolo_paths::directory::PrivateDirectory;

pub(crate) fn retain(parent: PrivateDirectory) -> Result<PrivateDirectory> {
    parent.verify_path()?;
    let path = yolo_paths::platform::platform_path(parent.path());
    if path == parent.path() {
        return Ok(parent);
    }
    let normalized = PrivateDirectory::open(&path)?;
    ensure!(
        normalized.same_identity(&parent),
        crate::constants::CONFLICT
    );
    parent.verify_path()?;
    normalized.verify_path()?;
    Ok(normalized)
}
