use crate::Repository;
use crate::constants::{BUILD_SYSTEM_PREFIX, PRODUCTION_SOURCE_LAYOUT_MESSAGE, SOURCE_DIRECTORY};
use std::collections::BTreeSet;
use std::path::Path;
use yolo_architecture_foundation::model::Violation;

pub fn inspect(
    repository: &Repository,
    production_sources: &BTreeSet<String>,
    violations: &mut Vec<Violation>,
) {
    for source in production_sources
        .iter()
        .filter(|source| source.starts_with(BUILD_SYSTEM_PREFIX))
    {
        let Some(package) = repository.source_package(source) else {
            continue;
        };
        let source_root = Path::new(&package).join(SOURCE_DIRECTORY);
        if !Path::new(source).starts_with(source_root) {
            violations.push(Violation::file(source, PRODUCTION_SOURCE_LAYOUT_MESSAGE));
        }
    }
}
