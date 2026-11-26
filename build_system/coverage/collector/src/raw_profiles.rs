use crate::constants::RAW_PROFILE_EXTENSION;
use std::{
    io,
    path::{Path, PathBuf},
};

pub(crate) fn read(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot inspect {}: {error}", directory.display()))?;
    select(
        directory,
        entries.map(|entry| entry.map(|entry| entry.path())),
    )
}

pub(crate) fn select(
    directory: &Path,
    entries: impl IntoIterator<Item = io::Result<PathBuf>>,
) -> Result<Vec<PathBuf>, String> {
    let mut profiles = Vec::new();
    for entry in entries {
        let path = entry.map_err(|error| {
            format!(
                "cannot enumerate raw profiles in {}: {error}",
                directory.display()
            )
        })?;
        if path
            .extension()
            .is_some_and(|value| value == RAW_PROFILE_EXTENSION)
        {
            profiles.push(path);
        }
    }
    profiles.sort();
    if profiles.is_empty() {
        return Err("coverage run produced no raw profiles".to_owned());
    }
    Ok(profiles)
}
