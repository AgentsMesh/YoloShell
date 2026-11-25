mod constants;
mod dependencies;
mod entrypoint;

use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_target_ownership::Repository;

pub use constants::{DEPENDENCIES_ATTRIBUTE, PACKAGE as COMPOSITION_PACKAGE};
pub use dependencies::resolve;

pub fn inspect(files: &[SourceFile], violations: &mut Vec<Violation>) {
    if !files
        .iter()
        .any(|file| file.relative == constants::BUILD)
    {
        return;
    }
    let repository = Repository::new(files);
    entrypoint::inspect(&repository, violations);
    dependencies::plugin::inspect(&repository, violations);
}

#[cfg(test)]
#[path = "../unit_tests/mod.rs"]
mod tests;
