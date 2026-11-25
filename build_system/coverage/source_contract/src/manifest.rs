use crate::constants::DECLARATION_SOURCE_EXTENSION;
use crate::{DECLARATION_SOURCE_SETS, paths};
use std::collections::BTreeSet;
use std::path::{Component, Path};
use yolo_coverage_protocol::{digest_bytes, production_key};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceContract {
    Declarations,
    TestsOnly,
}

/// Validate source eligibility and reviewed content; the caller supplies source bytes.
/// Executed LLVM mappings are separate evidence and remain the collector's responsibility.
pub fn verify_manifest(
    manifest: &str,
    mut read_source: impl FnMut(&str) -> Result<Vec<u8>, String>,
) -> Result<SourceContract, String> {
    let mut production = BTreeSet::new();
    let mut rust_sources = BTreeSet::new();
    let mut only_tests = true;
    let mut declared_test = false;
    for line in manifest.lines() {
        let line = line.trim_end_matches('\r');
        let path = Path::new(line);
        let rust_source = path
            .extension()
            .is_some_and(|extension| extension == DECLARATION_SOURCE_EXTENSION);
        if rust_source
            && path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(
                "declaration-only source manifest contains an unnormalized path".to_owned(),
            );
        }
        if let Some(source) = production_key(line)?
            && !production.insert(source)
        {
            return Err("declaration-only source manifest contains duplicates".to_owned());
        }
        if rust_source {
            if !rust_sources.insert(line) {
                return Err("coverage source manifest contains duplicate Rust sources".to_owned());
            }
            only_tests &= paths::is_test_source(line, true)?;
            declared_test |= paths::is_test_source(line, false)?;
        } else {
            only_tests &= paths::ancillary(line);
        }
    }
    if production.is_empty() && only_tests && declared_test {
        return Ok(SourceContract::TestsOnly);
    }
    let expected = DECLARATION_SOURCE_SETS
        .iter()
        .find(|set| {
            production.len() == set.len()
                && set.iter().all(|(source, _)| production.contains(*source))
        })
        .ok_or_else(|| {
            "missing production mappings have no declaration-only source contract".to_owned()
        })?;
    for (source, digest) in *expected {
        let bytes = read_source(source)?;
        if digest_bytes(&bytes) != *digest {
            return Err(format!(
                "declaration-only source changed and requires review: {source}"
            ));
        }
    }
    Ok(SourceContract::Declarations)
}
