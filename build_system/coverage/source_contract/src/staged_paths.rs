use crate::constants::{DECLARATION_BAZEL_BINARY, DECLARATION_BAZEL_OUTPUT};
use std::collections::BTreeSet;
use yolo_coverage_protocol::production_key;

const SOURCE_DIRECTORY: &str = "src";
const GENERATED_DIRECTORY: &str = "generated";
const RUST_EXTENSION: &str = ".rs";
const CURRENT_DIRECTORY: &str = ".";
const PARENT_DIRECTORY: &str = "..";
const INVALID_STAGING: &str = "invalid staged coverage source path";
const INVALID_MANIFEST: &str = "invalid staged coverage source manifest";

/// Recognize an alias for admission only; never use it as a coverage source key.
pub(crate) fn candidate(normalized: &str) -> Result<Option<String>, String> {
    let mut parts = normalized.split('/');
    if parts.next() != Some(DECLARATION_BAZEL_OUTPUT) {
        return Ok(None);
    }
    if !normalized_path(normalized)
        || parts.next().is_none()
        || parts.next() != Some(DECLARATION_BAZEL_BINARY)
    {
        return Err(INVALID_STAGING.into());
    }
    let source = parts.collect::<Vec<_>>().join("/");
    if source.is_empty() {
        return Err(INVALID_STAGING.into());
    }
    Ok(handwritten_production(&source)?.then_some(source))
}

pub(crate) fn declared(manifest: &str) -> Result<BTreeSet<String>, String> {
    let mut sources = BTreeSet::new();
    let mut rust_sources = BTreeSet::new();
    for line in manifest.lines() {
        let source = line.trim_end_matches('\r');
        if source.contains('\0') {
            return Err(INVALID_MANIFEST.into());
        }
        if !source.ends_with(RUST_EXTENSION) {
            continue;
        }
        if !normalized_path(source) || !rust_sources.insert(source) {
            return Err(INVALID_MANIFEST.into());
        }
        if handwritten_production(source)? {
            sources.insert(source.to_owned());
        }
    }
    if rust_sources.is_empty() {
        return Err(INVALID_MANIFEST.into());
    }
    Ok(sources)
}

fn handwritten_production(source: &str) -> Result<bool, String> {
    let parts = source.split('/').collect::<Vec<_>>();
    Ok(source.ends_with(RUST_EXTENSION)
        && parts.contains(&SOURCE_DIRECTORY)
        && !parts.contains(&GENERATED_DIRECTORY)
        && production_key(source)?.is_some())
}

pub(crate) fn normalized_path(path: &str) -> bool {
    !path.contains('\0')
        && path
            .split('/')
            .all(|part| !matches!(part, "" | CURRENT_DIRECTORY | PARENT_DIRECTORY))
}
