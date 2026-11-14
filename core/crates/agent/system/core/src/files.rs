use super::constants::{
    JSONL_EXTENSION, MAX_EVIDENCE_DEPTH, MAX_EVIDENCE_FILES, MAX_EVIDENCE_PENDING_DIRECTORIES,
    MAX_EVIDENCE_SCAN_ENTRIES,
};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileFingerprint {
    pub modified_ms: i64,
    pub len: u64,
}

pub fn collect_jsonl(root: &Path) -> Result<Vec<PathBuf>> {
    collect_jsonl_bounded(
        root,
        MAX_EVIDENCE_DEPTH,
        MAX_EVIDENCE_FILES,
        MAX_EVIDENCE_SCAN_ENTRIES,
        MAX_EVIDENCE_PENDING_DIRECTORIES,
    )
}

fn collect_jsonl_bounded(
    root: &Path,
    max_depth: usize,
    max_files: usize,
    max_entries: usize,
    max_pending: usize,
) -> Result<Vec<PathBuf>> {
    if max_files == 0 {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    let mut pending = vec![(root.to_path_buf(), 0_usize)];
    let mut scanned = 0_usize;
    while let Some((directory, depth)) = pending.pop() {
        let Some(entries) = read_directory(&directory)? else {
            continue;
        };
        for entry in entries {
            anyhow::ensure!(
                scanned < max_entries,
                "agent evidence scan entry limit reached"
            );
            scanned += 1;
            let entry = entry.context("read agent evidence directory entry")?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .context("inspect agent evidence directory entry")?;
            if file_type.is_dir() && depth < max_depth {
                anyhow::ensure!(
                    pending.len() < max_pending,
                    "agent evidence pending directory limit reached"
                );
                pending.push((path, depth + 1));
            } else if file_type.is_file()
                && path.extension().and_then(|value| value.to_str()) == Some(JSONL_EXTENSION)
            {
                files.push(path);
                if files.len() >= max_files {
                    return Ok(files);
                }
            }
        }
    }
    Ok(files)
}

fn read_directory(path: &Path) -> Result<Option<fs::ReadDir>> {
    match fs::read_dir(path) {
        Ok(entries) => Ok(Some(entries)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).context("read agent evidence directory"),
    }
}

pub fn fingerprint(path: &Path) -> Option<FileFingerprint> {
    let metadata = fs::metadata(path).ok()?;
    let modified_ms = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX);
    Some(FileFingerprint {
        modified_ms,
        len: metadata.len(),
    })
}

pub fn same_directory(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/files/edge_tests.rs"]
mod edge_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/files/tests.rs"]
mod tests;
