use super::declarations::Declaration;
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

const DIRECTORY_ROOT: &str = "mod";
const EMPTY_EXTENSION: &str = "";
const CONVENTIONAL_LIBRARY_ROOT: &str = "lib";
const CONVENTIONAL_BINARY_ROOT: &str = "main";
const PORTABLE_SEPARATOR: &str = "/";
const RUST_SUFFIX: &str = "rs";

pub(super) fn target(
    parent: &str,
    module_search_directory: Option<&str>,
    declaration: &Declaration,
    ancestors: &[&str],
    available: &BTreeSet<String>,
    crate_root: Option<&str>,
) -> Option<String> {
    let parent = Path::new(parent);
    let module_base = module_search_directory
        .map(PathBuf::from)
        .or_else(|| module_directory(parent, crate_root == parent.to_str()))?;
    let scoped = ancestors
        .iter()
        .fold(module_base, |path, name| path.join(name));
    let candidates = if let Some(path) = &declaration.path {
        let base = if ancestors.is_empty() {
            parent.parent()?.to_path_buf()
        } else {
            scoped.clone()
        };
        vec![base.join(path)]
    } else {
        vec![
            scoped.join(&declaration.name).with_extension(RUST_SUFFIX),
            scoped
                .join(&declaration.name)
                .join(format!("{DIRECTORY_ROOT}.{RUST_SUFFIX}")),
        ]
    };
    candidates
        .into_iter()
        .filter_map(normalized)
        .find(|candidate| available.contains(candidate))
}

pub(super) fn included(parent: &str, literal: &str) -> Option<String> {
    normalized(Path::new(parent).parent()?.join(literal))
}

pub(super) fn containing_directory(relative: &str) -> Option<String> {
    normalized(Path::new(relative).parent()?.to_path_buf())
}

pub(super) fn conventional_crate_root(relative: &str) -> bool {
    Path::new(relative)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| matches!(stem, CONVENTIONAL_LIBRARY_ROOT | CONVENTIONAL_BINARY_ROOT))
}

fn module_directory(parent: &Path, crate_root: bool) -> Option<PathBuf> {
    let stem = parent.file_stem()?.to_str()?;
    if crate_root || stem == DIRECTORY_ROOT {
        parent.parent().map(Path::to_path_buf)
    } else {
        Some(parent.with_extension(EMPTY_EXTENSION))
    }
}

fn normalized(path: PathBuf) -> Option<String> {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => result.push(value),
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop().then_some(())?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(
        result
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, PORTABLE_SEPARATOR),
    )
}
