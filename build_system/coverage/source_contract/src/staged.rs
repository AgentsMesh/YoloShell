use crate::staged_paths;
use std::collections::BTreeSet;

const UNREMAPPED_SOURCE: &str = "unremapped declared production coverage source";

/// Reject staged aliases of declared handwritten production sources before filtering.
/// Paths must already be normalized relative to the execution root. No path is
/// remapped or admitted here; the manifest is loaded only for staged candidates.
pub fn reject_staged_production<I, P, F>(paths: I, load_manifest: F) -> Result<(), String>
where
    I: IntoIterator<Item = P>,
    P: AsRef<str>,
    F: FnOnce() -> Result<String, String>,
{
    let mut candidates = BTreeSet::new();
    for path in paths {
        if let Some(source) = staged_paths::candidate(path.as_ref())? {
            candidates.insert(source);
        }
    }
    if candidates.is_empty() {
        return Ok(());
    }
    let declared = staged_paths::declared(&load_manifest()?)?;
    if let Some(source) = candidates.intersection(&declared).next() {
        return Err(format!("{UNREMAPPED_SOURCE}: {source}"));
    }
    Ok(())
}
