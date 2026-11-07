use anyhow::{Result, ensure};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const NON_REGULAR_FIXTURE: &str = "copy fixture contains a non-regular object";

pub(super) fn copy_directory(source: &Path, target: &Path) -> Result<()> {
    std::fs::create_dir(target)?;
    std::fs::set_permissions(
        target,
        std::fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
    )?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let target = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else {
            ensure!(entry.file_type()?.is_file(), NON_REGULAR_FIXTURE);
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
