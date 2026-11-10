use crate::constants::{
    BUILD_FILE_NAMES, FIXTURE_DIRECTORY, GENERATED_DIRECTORY, INTEGRATION_TEST_DIRECTORY,
    TEST_DIRECTORY_SUFFIX, TEST_SUPPORT_DIRECTORY, TEST_SUPPORT_PACKAGE_SUFFIX,
    UNIT_TEST_DIRECTORY,
};
use std::path::{Component, Path, PathBuf};

pub fn package(path: &str) -> String {
    Path::new(path).parent().map(normalized).unwrap_or_default()
}
pub fn is_build_file(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| BUILD_FILE_NAMES.contains(&name))
}

pub fn label(package: &str, name: &str) -> String {
    if package.is_empty() {
        format!("//:{name}")
    } else {
        format!("//{package}:{name}")
    }
}

pub fn label_parts(current_package: &str, value: &str) -> Option<(String, String)> {
    if value.starts_with('@') {
        return None;
    }
    if let Some(local) = value.strip_prefix(':') {
        return Some((current_package.to_owned(), local.to_owned()));
    }
    let absolute = value.strip_prefix("//")?;
    let (package, name) = absolute
        .split_once(':')
        .map_or((absolute, file_name(absolute)), |pair| pair);
    Some((package.to_owned(), name.to_owned()))
}

pub fn source_path(current_package: &str, value: &str) -> Option<String> {
    if value.starts_with('@') {
        return None;
    }
    if value.starts_with("//") && !value.contains(':') {
        return None;
    }
    if let Some((package, name)) = label_parts(current_package, value) {
        return name.ends_with(".rs").then(|| join(&package, &name));
    }
    value.ends_with(".rs").then(|| join(current_package, value))
}

pub fn resolve_relative(source: &str, relative: &str) -> Option<String> {
    let parent = Path::new(source).parent()?;
    normalize_path(&parent.join(relative)).map(|path| normalized(&path))
}

pub fn is_test_path(path: &str) -> bool {
    Path::new(path).components().any(|component| {
        matches!(component, Component::Normal(name) if name == UNIT_TEST_DIRECTORY
            || name == INTEGRATION_TEST_DIRECTORY || name == TEST_SUPPORT_DIRECTORY
            || name == FIXTURE_DIRECTORY
            || name == GENERATED_DIRECTORY
            || name.to_str().is_some_and(|value| value.ends_with(TEST_DIRECTORY_SUFFIX)
                || value.ends_with(TEST_SUPPORT_PACKAGE_SUFFIX)))
    })
}

pub fn is_test_owned_path(path: &str) -> bool {
    Path::new(path).components().any(|component| {
        matches!(component, Component::Normal(name) if name == UNIT_TEST_DIRECTORY
            || name == INTEGRATION_TEST_DIRECTORY || name == TEST_SUPPORT_DIRECTORY
            || name == FIXTURE_DIRECTORY
            || name.to_str().is_some_and(|value| value.ends_with(TEST_DIRECTORY_SUFFIX)
                || value.ends_with(TEST_SUPPORT_PACKAGE_SUFFIX)))
    })
}

fn join(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_owned()
    } else {
        format!("{parent}/{child}")
    }
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn normalize_path(path: &Path) -> Option<PathBuf> {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => result.push(value),
            Component::CurDir => {}
            Component::ParentDir if result.pop() => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(result)
}

fn normalized(path: &Path) -> String {
    path.to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}
