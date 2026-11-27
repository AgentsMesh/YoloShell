use crate::constants::DECLARATION_FILE_LIMIT;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

pub(crate) fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let mut file = File::open(path).map_err(|error| {
        format!(
            "cannot read declaration evidence {}: {error}",
            path.display()
        )
    })?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(DECLARATION_FILE_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read declaration evidence: {error}"))?;
    if bytes.len() as u64 > DECLARATION_FILE_LIMIT {
        return Err("declaration evidence exceeds its size bound".to_owned());
    }
    Ok(bytes)
}

pub(crate) fn runfile_root(
    execution_root: &Path,
    runfiles: &Path,
    workspace: &Path,
) -> Result<PathBuf, String> {
    let mut components = workspace.components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err(
            "declaration-only workspace must be one runfiles directory component".to_owned(),
        );
    }
    Ok(execution_root.join(runfiles).join(workspace))
}
