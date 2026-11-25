use super::{constants, metadata};
use anyhow::{Context, Result, ensure};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn evidence_files(root: &Path, cwd: &Path) -> Result<Vec<PathBuf>> {
    let mut directories = project_directories(root, cwd)?;
    directories.sort();
    directories.dedup();
    let mut files = Vec::new();
    for directory in directories {
        collect_directory(&directory, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn project_directories(root: &Path, cwd: &Path) -> Result<Vec<PathBuf>> {
    let mut working_directories = vec![cwd.to_path_buf()];
    if let Ok(canonical) = cwd.canonicalize()
        && canonical != cwd
    {
        working_directories.push(canonical);
    }
    let mut directories = Vec::new();
    for working_directory in working_directories {
        let ancestors = working_directory.ancestors().collect::<Vec<_>>();
        ensure!(
            ancestors.len() <= constants::MAX_PROJECT_ANCESTOR_SCOPES,
            constants::PROJECT_ANCESTOR_LIMIT_ERROR
        );
        directories.extend(
            ancestors
                .into_iter()
                .map(|path| project_directory(root, path)),
        );
    }
    Ok(directories)
}

pub(super) fn project_directory(root: &Path, cwd: &Path) -> PathBuf {
    root.join(project_key(cwd))
}

fn project_key(cwd: &Path) -> String {
    cwd.to_string_lossy()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                constants::PROJECT_PATH_REPLACEMENT
            }
        })
        .collect()
}

fn collect_directory(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).context(constants::EVIDENCE_DIRECTORY_READ_ERROR),
    };
    for (index, entry) in entries.enumerate() {
        ensure!(
            index < constants::MAX_PROJECT_DIRECTORY_ENTRIES,
            constants::DISCOVERY_ENTRY_LIMIT_ERROR
        );
        let entry = entry.context(constants::EVIDENCE_ENTRY_READ_ERROR)?;
        let file_type = entry
            .file_type()
            .context(constants::EVIDENCE_ENTRY_INSPECT_ERROR)?;
        if file_type.is_file() && metadata::main_transcript_path(&entry.path()) {
            ensure!(
                files.len() < constants::MAX_PROJECT_TRANSCRIPTS,
                constants::DISCOVERY_FILE_LIMIT_ERROR
            );
            files.push(entry.path());
        }
    }
    Ok(())
}
