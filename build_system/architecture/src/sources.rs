use crate::constants::{IGNORED_ROOT_DIRECTORIES, TEXT_EXTENSIONS, TEXT_FILE_NAMES};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use yolo_architecture_repository::repository_rules;
use yolo_architecture_source_policy::source_policy::{
    is_retired_frontend_directory, is_retired_frontend_file,
};

pub use yolo_architecture_foundation::SourceFile;

#[cfg(test)]
#[path = "../unit_tests/source_loading_tests.rs"]
mod loading_tests;

pub fn load(root: &Path) -> io::Result<Vec<SourceFile>> {
    let mut paths = Vec::new();
    let mut visited = BTreeSet::new();
    visit(root, root, &mut paths, &mut visited)?;
    paths.sort();
    paths
        .into_iter()
        .map(|absolute| {
            let relative = absolute.strip_prefix(root).unwrap_or(&absolute);
            let relative = relative
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            let text = fs::read_to_string(&absolute)?;
            Ok(SourceFile {
                absolute,
                relative,
                text,
            })
        })
        .collect()
}

pub fn retired_directories(root: &Path) -> io::Result<Vec<String>> {
    Ok(directories(root)?
        .into_iter()
        .filter(|relative| is_retired_frontend_directory(Path::new(relative)))
        .collect())
}

pub fn directories(root: &Path) -> io::Result<Vec<String>> {
    let mut result = Vec::new();
    let mut visited = BTreeSet::new();
    visit_directories(root, root, &mut result, &mut visited)?;
    result.sort();
    Ok(result)
}

fn visit(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
    visited: &mut BTreeSet<PathBuf>,
) -> io::Result<()> {
    let canonical = directory.canonicalize()?;
    if !visited.insert(canonical) {
        return Ok(());
    }
    for result in fs::read_dir(directory)? {
        let entry = result?;
        let path = entry.path();
        let kind = if entry.file_type()?.is_symlink() {
            match fs::metadata(&path) {
                Ok(metadata) => metadata.file_type(),
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            }
        } else {
            entry.file_type()?
        };
        if kind.is_dir() {
            if !ignored(root, &path) {
                visit(root, &path, files, visited)?;
            }
        } else if kind.is_file() && scanned(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn visit_directories(
    root: &Path,
    directory: &Path,
    result: &mut Vec<String>,
    visited: &mut BTreeSet<PathBuf>,
) -> io::Result<()> {
    let canonical = directory.canonicalize()?;
    if !visited.insert(canonical) {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let metadata = match fs::metadata(&path) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error),
        };
        if !metadata.is_dir() {
            continue;
        }
        result.push(relative(root, &path));
        if !ignored(root, &path) {
            visit_directories(root, &path, result, visited)?;
        }
    }
    Ok(())
}

fn ignored(root: &Path, path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if root_entry(root, path)
        && (IGNORED_ROOT_DIRECTORIES.contains(&name) || name.starts_with("bazel-"))
    {
        return true;
    }
    is_retired_frontend_directory(path) || path == root
}

fn root_entry(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    let mut components = relative.components();
    components.next().is_some() && components.next().is_none()
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

const SHEBANG_PREFIX: &[u8] = b"#!";
const SHEBANG_PROBE_BYTES: u64 = 256;

fn scanned(path: &Path) -> bool {
    if is_retired_frontend_file(path) || repository_rules::is_policy_file(path) {
        return true;
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if TEXT_FILE_NAMES.contains(&name) {
        return true;
    }
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| TEXT_EXTENSIONS.contains(&extension))
        || extensionless_shebang(path)
}

fn extensionless_shebang(path: &Path) -> bool {
    if path.extension().is_some() {
        return false;
    }
    let Ok(file) = fs::File::open(path) else {
        return false;
    };
    let mut prefix = Vec::new();
    file.take(SHEBANG_PROBE_BYTES)
        .read_to_end(&mut prefix)
        .is_ok()
        && prefix.starts_with(SHEBANG_PREFIX)
}
