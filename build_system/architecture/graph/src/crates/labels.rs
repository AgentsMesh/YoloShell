use super::repositories::{self, Evidence, Repository};
use std::path::Path;
use yolo_architecture_policy_contracts::build_constants as vocabulary;

const ABSOLUTE_LABEL_PREFIX: &str = "//";
const APPARENT_MAIN_REPOSITORY_PREFIX: &str = "@//";
const BINARY_DEFAULT_ROOT: &str = "main.rs";
const BUILD_DEFINITION_SUFFIX: &str = ".bzl";
const BUILD_FILES: &[&str] = &["BUILD", "BUILD.bazel"];
const CANONICAL_MAIN_REPOSITORY_PREFIX: &str = "@@//";
const EXTERNAL_REPOSITORY_PREFIX: char = '@';
const LABEL_SEPARATOR: char = ':';
const LIBRARY_DEFAULT_ROOT: &str = "lib.rs";
const PATH_SEPARATOR: &str = "/";
const RUST_SOURCE_SUFFIX: &str = ".rs";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Resolution {
    Resolved(String),
    External,
    Uncertain,
}

pub(super) fn default_root(rule: &str) -> Option<&'static str> {
    match rule {
        vocabulary::RUST_BINARY_RULE => Some(BINARY_DEFAULT_ROOT),
        vocabulary::RUST_LIBRARY_RULE
        | vocabulary::RUST_STATIC_LIBRARY_RULE
        | vocabulary::RUST_SHARED_LIBRARY_RULE
        | vocabulary::RUST_PROC_MACRO_RULE
        | vocabulary::RUST_TEST_RULE => Some(LIBRARY_DEFAULT_ROOT),
        _ => None,
    }
}

pub(super) fn build_definition(relative: &str) -> bool {
    Path::new(relative)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| BUILD_FILES.contains(&name) || name.ends_with(BUILD_DEFINITION_SUFFIX))
}

#[cfg(test)]
pub(super) fn resolve(package: &str, label: &str) -> Resolution {
    resolve_with(package, label, &Evidence::Known(String::new()))
}

pub(super) fn resolve_with(package: &str, label: &str, main: &Evidence) -> Resolution {
    if let Some(value) = label.strip_prefix(CANONICAL_MAIN_REPOSITORY_PREFIX) {
        return absolute(value);
    }
    if let Some(value) = label.strip_prefix(APPARENT_MAIN_REPOSITORY_PREFIX) {
        return absolute(value);
    }
    if label.starts_with(EXTERNAL_REPOSITORY_PREFIX) {
        return match repositories::classify(label, main) {
            Repository::Main(value) => absolute(value),
            Repository::External => Resolution::External,
            Repository::Uncertain => Resolution::Uncertain,
        };
    }
    if let Some(value) = label.strip_prefix(ABSOLUTE_LABEL_PREFIX) {
        return absolute(value);
    }
    relative(package, label)
}

pub(super) fn infer(
    package: &str,
    labels: &[String],
    default: &str,
    name: Option<&str>,
    crate_name: Option<&str>,
    main: &Evidence,
) -> Resolution {
    if labels.len() == 1 {
        return resolve_with(package, &labels[0], main);
    }
    if labels.is_empty() {
        return Resolution::Uncertain;
    }
    let resolved = labels
        .iter()
        .map(|label| resolve_with(package, label, main))
        .collect::<Vec<_>>();
    if resolved.iter().all(|value| value == &Resolution::External) {
        return Resolution::External;
    }
    let paths = resolved
        .into_iter()
        .map(|value| match value {
            Resolution::Resolved(path) => Some(path),
            Resolution::External | Resolution::Uncertain => None,
        })
        .collect::<Option<Vec<_>>>();
    let Some(paths) = paths else {
        return Resolution::Uncertain;
    };
    for basename in basenames(default, name, crate_name) {
        if let Some(result) = shortest(&paths, &basename) {
            return result;
        }
    }
    Resolution::Uncertain
}

fn absolute(label: &str) -> Resolution {
    let Some((package, target)) = single_split(label) else {
        return Resolution::Uncertain;
    };
    resolved(package, target)
}

fn relative(package: &str, label: &str) -> Resolution {
    if label.starts_with(PATH_SEPARATOR) {
        return Resolution::Uncertain;
    }
    if let Some(target) = label.strip_prefix(LABEL_SEPARATOR) {
        return resolved(package, target);
    }
    if label.contains(LABEL_SEPARATOR) {
        let Some((child, target)) = single_split(label) else {
            return Resolution::Uncertain;
        };
        return resolved(&join(package, child), target);
    }
    resolved(package, label)
}

fn single_split(label: &str) -> Option<(&str, &str)> {
    let (package, target) = label.split_once(LABEL_SEPARATOR)?;
    (!target.contains(LABEL_SEPARATOR)).then_some((package, target))
}

fn resolved(package: &str, target: &str) -> Resolution {
    if target.is_empty() {
        return Resolution::Uncertain;
    }
    let path = join(package, target);
    if valid(&path) {
        Resolution::Resolved(path)
    } else {
        Resolution::Uncertain
    }
}

fn join(left: &str, right: &str) -> String {
    [left, right]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(PATH_SEPARATOR)
}

fn valid(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && path
            .split(PATH_SEPARATOR)
            .all(|part| !part.is_empty() && !matches!(part, "." | ".."))
}

fn basenames(default: &str, name: Option<&str>, crate_name: Option<&str>) -> Vec<String> {
    std::iter::once(default.to_owned())
        .chain(name.map(|value| format!("{value}{RUST_SOURCE_SUFFIX}")))
        .chain(crate_name.map(|value| format!("{value}{RUST_SOURCE_SUFFIX}")))
        .collect()
}

fn shortest(paths: &[String], basename: &str) -> Option<Resolution> {
    let mut matches = paths
        .iter()
        .filter(|path| Path::new(path).file_name().and_then(|name| name.to_str()) == Some(basename))
        .collect::<Vec<_>>();
    let length = matches
        .iter()
        .filter_map(|path| {
            Path::new(path)
                .parent()
                .and_then(Path::to_str)
                .map(str::len)
        })
        .min()?;
    matches.retain(|path| {
        Path::new(path)
            .parent()
            .and_then(Path::to_str)
            .map(str::len)
            == Some(length)
    });
    match matches.as_slice() {
        [path] => Some(Resolution::Resolved((*path).clone())),
        [] => None,
        _ => Some(Resolution::Uncertain),
    }
}
