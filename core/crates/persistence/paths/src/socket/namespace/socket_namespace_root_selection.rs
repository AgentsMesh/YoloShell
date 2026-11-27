use super::*;
use anyhow::{Context, Result};

const OVERRIDE_OPEN_CONTEXT: &str = "open configured socket namespace root";
const ROOT_PATH_ERROR: &str = "configured socket namespace root cannot be filesystem root";

pub(super) struct SocketRootSelection {
    pub(super) path: PathBuf,
    pub(super) overridden: bool,
}

impl SocketRootSelection {
    pub(super) fn new(effective_user_id: u32, root: Option<&Path>) -> Result<Self> {
        let selection = Self::unchecked(effective_user_id, root.map(Path::to_path_buf));
        if selection.overridden {
            crate::paths::validation::validate_absolute(&selection.path)?;
            anyhow::ensure!(selection.path.parent().is_some(), ROOT_PATH_ERROR);
        }
        Ok(selection)
    }

    pub(super) fn unchecked(effective_user_id: u32, root: Option<PathBuf>) -> Self {
        match root {
            Some(path) => Self {
                path,
                overridden: true,
            },
            None => Self::default(effective_user_id),
        }
    }

    pub(super) fn default(effective_user_id: u32) -> Self {
        Self {
            path: default_private_root(effective_user_id),
            overridden: false,
        }
    }
}

pub(super) fn open(path: &Path, overridden: bool) -> Result<PrivateDirectory> {
    if overridden {
        crate::paths::validation::validate_absolute(path)?;
        return PrivateDirectory::open(path).context(OVERRIDE_OPEN_CONTEXT);
    }
    PrivateDirectory::ensure(path)
}
