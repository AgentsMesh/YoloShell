use crate::constants::*;
use anyhow::{Context, Result, ensure};
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Helpers {
    pub executable: PathBuf,
}

impl Helpers {
    pub fn discover(executable: &Path) -> Result<Self> {
        let executable = executable.canonicalize().context(PACKAGE_INVALID)?;
        require_executable(&executable)?;
        Ok(Self { executable })
    }
}

fn require_executable(path: &Path) -> Result<()> {
    let metadata = path.symlink_metadata().context(PACKAGE_INVALID)?;
    let mode = metadata.permissions().mode();
    ensure!(
        metadata.file_type().is_file()
            && mode & EXECUTABLE_MASK != 0
            && mode & WRITABLE_BY_OTHERS_MASK == 0,
        PACKAGE_INVALID
    );
    Ok(())
}
